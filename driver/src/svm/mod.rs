extern crate alloc;

use crate::nt::include::Context;
use crate::nt::processor::{processor_count, ProcessorExecutor};
use crate::svm::data::msr_bitmap::EFER_SVME;
use crate::svm::data::processor::ProcessorDataWrapper;
use crate::svm::data::shared_data::SharedData;
use crate::svm::vmexit::CPUID_DEVIRTUALIZE;
use crate::svm::vmlaunch::launch_vm;
use crate::{dbg_break, support, KeBugCheck, MANUALLY_INITIATED_CRASH};
use alloc::vec::Vec;
use x86::cpuid::{cpuid, CpuId, Hypervisor};
use x86::msr::{rdmsr, wrmsr, IA32_EFER};

pub mod data;
pub mod events;
pub mod paging;
pub mod vmcb;
pub mod vmexit;
pub mod vmlaunch;

pub struct Processors {
    shard_data: SharedData,
    processors: Vec<Processor>,
}

impl Processors {
    /// Creates new instance for all the processors on the system.
    pub fn new() -> Option<Self> {
        if !support::is_svm_supported() {
            log::error!("SVM is not supported");
            return None;
        }

        let processors = (0..processor_count())
            .filter_map(Processor::new)
            .collect::<Vec<_>>();
        log::info!("Found {} processors", processors.len());

        Some(Self {
            shard_data: SharedData::new()?,
            processors,
        })
    }

    pub fn virtualize(&mut self) -> bool {
        for processor in self.processors.iter_mut() {
            if !processor.virtualize(&self.shard_data) {
                log::error!("Failed to virtualize processor {}", processor.id());
                return false;
            }
        }

        true
    }

    pub fn devirtualize(&mut self) -> bool {
        for processor in self.processors.iter_mut() {
            if !processor.devirtualize() {
                log::error!("Failed to devirtualize processor {}", processor.id());
                return false;
            }
        }

        true
    }
}

pub struct Processor {
    /// The index of the processor.
    index: u32,

    data: ProcessorDataWrapper,
}

impl Processor {
    pub fn new(index: u32) -> Option<Self> {
        log::trace!("Creating processor {}", index);

        Some(Self {
            index,
            data: ProcessorDataWrapper::new()?,
        })
    }

    /// Checks whether the current process is already virtualized.
    ///
    /// This is done by comparing the value of cpuid leaf 0x40000000. The cpuid
    /// vmexit has to return the correct value to be able to use this.
    pub fn is_virtualized(&self) -> bool {
        CpuId::new()
            .get_hypervisor_info()
            .map(|hv_info| match hv_info.identify() {
                Hypervisor::Unknown(ebx, ecx, edx) => {
                    log::info!("Found unknown hypervisor: {:x} {:x} {:x}", ebx, ecx, edx);

                    ebx == 0x42 && ecx == 0x42 && edx == 0x42
                }
                _ => false,
            })
            .unwrap_or_default()
    }

    pub fn virtualize(&mut self, shared_data: &SharedData) -> bool {
        let Some(executor) = ProcessorExecutor::switch_to_processor(self.index) else {
            log::error!("Failed to switch to processor");
            return false
        };

        // Based on this: https://github.com/tandasat/SimpleSvm/blob/master/SimpleSvm/SimpleSvm.cpp#L1137

        // IMPORTANT: We have to capture the context right here, so that `launch_vm` continues the
        // execution of the current process at this point of time. If we don't do this,
        // weird things will happen since we will execute the guest at another point.
        //
        // This also makes sense why `vmsave` was crashing inside `prepare_for_virtualization`. It
        // obviously entered the guest state and tried to execute from there on. And because of that,
        // everything that happened afterwards is just undefined behaviour.
        //
        // Literally wasted like a whole day just because of this 1 line.
        //
        let context = Context::capture();

        log::info!("After context has been captured");
        dbg_break!();

        // Check if already virtualized.
        //
        if !self.is_virtualized() {
            // Attempt to virtualize the processor
            //

            // Enable SVM by setting EFER.SVME.
            let msr = unsafe { rdmsr(IA32_EFER) } | EFER_SVME;
            unsafe { wrmsr(IA32_EFER, msr) };

            // Setup vmcb
            //
            log::info!("Prepared vmcb for virtualization");
            self.data.prepare_for_virtualization(shared_data, context);

            // Launch vm
            // https://github.com/tandasat/SimpleSvm/blob/master/SimpleSvm/x64.asm#L78
            //
            log::info!("Launching vm");

            // TODO: Figure out why it's crashing after vmlaunch.
            // Why?
            // 1. The pointer is invalid.
            // 2. The npt is not setup correctly.
            // 3. ???

            let host_rsp = unsafe { &(*self.data.data).host_stack_layout.guest_vmcb_pa };
            let host_rsp = host_rsp as *const u64 as u64;
            unsafe { launch_vm(host_rsp) };

            // We should never continue the guest execution here.
            //
            dbg_break!();
            unsafe { KeBugCheck(MANUALLY_INITIATED_CRASH) };
        }

        log::warn!("Processor {} is now virtualized", self.index);

        core::mem::drop(executor);

        true
    }

    pub fn devirtualize(&self) -> bool {
        let Some(executor) = ProcessorExecutor::switch_to_processor(self.index) else {
            log::error!("Failed to switch to processor");
            return false
        };

        // Already devirtualized? Then we don't need to do anything.
        //
        let result = cpuid!(CPUID_DEVIRTUALIZE, CPUID_DEVIRTUALIZE);
        if result.ecx != 0xDEADBEEF {
            return true;
        }

        log::info!("Processor {} has been devirtualized", self.index);

        core::mem::drop(executor);

        true
    }

    pub fn id(&self) -> u32 {
        self.index
    }
}
