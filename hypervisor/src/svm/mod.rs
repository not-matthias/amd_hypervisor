extern crate alloc;

use crate::{
    debug::dbg_break,
    svm::{
        data::{processor_data::ProcessorData, shared_data::SharedData},
        msr::EFER_SVME,
        vmexit::{cpuid::CPUID_DEVIRTUALIZE, VmExitHandler},
        vmlaunch::launch_vm,
    },
    utils::{
        nt::{Context, KeBugCheck, MANUALLY_INITIATED_CRASH},
        processor::{processor_count, ProcessorExecutor},
    },
};
use alloc::{boxed::Box, vec::Vec};
use core::lazy::OnceCell;
use x86::{
    cpuid::cpuid,
    msr::{rdmsr, wrmsr, IA32_EFER},
};

pub mod data;
pub mod events;
pub mod msr;
pub mod paging;
pub mod support;
pub mod vmcb;
pub mod vmexit;
pub mod vmlaunch;

#[derive(Hash, Ord, PartialOrd, Eq, PartialEq, Debug, Copy, Clone)]
pub enum VmExitType {
    /// Cpuid instruction with eax = {0}
    Cpuid(u32),

    /// Rdmsr/Wrmsr with msr = {0}
    Msr(u32),
    Rdmsr(u32),
    Wrmsr(u32),

    NestedPageFault,
    Breakpoint,
    Rdtsc,
    Rdtscp,
    Vmcall,
}

pub struct Hypervisor<T = ()> {
    shared_data: Box<SharedData>,
    processors: Vec<Processor<T>>,
}

impl<T: Default> Hypervisor<T> {
    /// Creates new instance for all the processors on the system.
    pub fn new() -> Option<Self> {
        if !support::is_svm_supported() {
            log::error!("SVM is not supported");
            return None;
        }

        let mut processors = Vec::new();
        for i in 0..processor_count() {
            processors.push(Processor::new(i)?);
        }
        log::info!("Found {} processors", processors.len());

        Some(Self {
            shared_data: SharedData::new()?,
            processors,
        })
    }

    /// Adds the specified handler.
    ///
    /// Note: If a handler is already registered for the specified type, it will
    /// be replaced.
    #[must_use]
    pub fn with_handler(mut self, vmexit_type: VmExitType, handler: VmExitHandler) -> Self {
        // If it's an msr, we also have to set the permission in the bitmap.
        //
        match &vmexit_type {
            VmExitType::Msr(msr) => self.shared_data.msr_bitmap.hook_msr(*msr),
            VmExitType::Rdmsr(msr) => self.shared_data.msr_bitmap.hook_rdmsr(*msr),
            VmExitType::Wrmsr(msr) => self.shared_data.msr_bitmap.hook_wrmsr(*msr),
            _ => {}
        }

        if vmexit::VMEXIT_HANDLERS
            .write()
            .insert(vmexit_type, handler)
            .is_some()
        {
            log::warn!(
                "Handler for {:?} was overwritten. Is this on purpose?",
                vmexit_type
            );
        }

        self
    }

    pub fn virtualize(&mut self) -> bool {
        log::info!("Virtualizing processors");

        let mut status = true;
        for processor in self.processors.iter_mut() {
            // NOTE: We have to execute this in the loop and can't do it in the `virtualize`
            // function for some reason. If we do, an access violation occurs.
            //
            let Some(executor) = ProcessorExecutor::switch_to_processor(processor.id()) else {
                log::error!("Failed to switch to processor");
                status = false;
                break;
            };

            if !processor.virtualize(self.shared_data.as_mut()) {
                log::error!("Failed to virtualize processor {}", processor.id());

                status = false;
                break;
            }

            core::mem::drop(executor);
        }

        // Devirtualize if the virtualization failed.
        //
        if !status {
            log::info!("Failed to virtualize processors, devirtualizing.");
            self.devirtualize();
        }

        true
    }

    pub fn devirtualize(&mut self) -> bool {
        let mut status = true;
        for processor in self.processors.iter_mut() {
            let Some(executor) = ProcessorExecutor::switch_to_processor(processor.id()) else {
                log::error!("Failed to switch to processor");
                status = false;
                continue;
            };

            if !processor.devirtualize() {
                log::error!("Failed to devirtualize processor {}", processor.id());
                status = false;
            }

            core::mem::drop(executor);
        }

        status
    }
}

pub struct Processor<T = ()> {
    /// The index of the processor.
    index: u32,

    processor_data: OnceCell<Box<ProcessorData<T>>>,
}

impl<T: Default> Processor<T> {
    pub fn new(index: u32) -> Option<Self> {
        log::trace!("Creating processor {}", index);

        Some(Self {
            index,
            processor_data: OnceCell::new(),
        })
    }

    pub fn virtualize(&mut self, shared_data: &mut SharedData) -> bool {
        log::info!("Virtualizing processor {}", self.index);

        // Based on this: https://github.com/tandasat/SimpleSvm/blob/master/SimpleSvm/SimpleSvm.cpp#L1137

        // IMPORTANT: We have to capture the context right here, so that `launch_vm`
        // continues the execution of the current process at this point of time.
        // If we don't do this, weird things will happen since we will execute
        // the guest at another point.
        //
        // This also makes sense why `vmsave` was crashing inside
        // `prepare_for_virtualization`. It obviously entered the guest state
        // and tried to execute from there on. And because of that, everything
        // that happened afterwards is just undefined behaviour.
        //
        // Literally wasted like a whole day just because of this 1 line.
        //
        log::info!("Capturing context");
        let context = Context::capture();

        // Check if already virtualized.
        //
        if !support::is_virtualized() {
            log::info!("Preparing for virtualization");

            support::set_virtualized();

            // Enable SVM by setting EFER.SVME.
            let msr = unsafe { rdmsr(IA32_EFER) } | EFER_SVME;
            unsafe { wrmsr(IA32_EFER, msr) };

            // Setup processor data and get host rsp.
            //
            let host_rsp = &self
                .processor_data
                .get_or_init(|| ProcessorData::new(shared_data, context))
                .host_stack_layout
                .guest_vmcb_pa as *const u64 as *mut u64;

            // Launch vm
            // https://github.com/tandasat/SimpleSvm/blob/master/SimpleSvm/x64.asm#L78
            //
            log::info!("Launching vm");
            unsafe { launch_vm(host_rsp) };

            // We should never continue the guest execution here.
            //
            dbg_break!();
            unsafe { KeBugCheck(MANUALLY_INITIATED_CRASH) };
        }

        true
    }

    pub fn devirtualize(&self) -> bool {
        // Already devirtualized? Then we don't need to do anything.
        //
        let result = cpuid!(CPUID_DEVIRTUALIZE);
        if result.ecx != 0xDEADBEEF {
            log::info!(
                "Ecx is not 0xDEADBEEF. Nothing to do. Ecx: {:x}",
                result.ecx
            );
            return true;
        }

        log::info!("Processor {} has been devirtualized", self.index);

        true
    }

    pub fn id(&self) -> u32 {
        self.index
    }
}
