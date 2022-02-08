use crate::{
    svm::{shared_data::SharedData, vcpu::Vcpu, vmexit::VmExitHandler},
    utils::processor::{processor_count, ProcessorExecutor},
};
use alloc::{boxed::Box, vec::Vec};

pub mod events;
pub mod msr_bitmap;
pub mod nested_page_table;
pub mod shared_data;
pub mod support;
pub mod utils;
pub mod vcpu;
pub mod vcpu_data;
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

pub struct Hypervisor {
    shared_data: Box<SharedData>,
    processors: Vec<Vcpu>,
}

impl Hypervisor {
    /// Creates new instance for all the processors on the system.
    pub fn new() -> Option<Self> {
        if !support::is_svm_supported() {
            log::error!("SVM is not supported");
            return None;
        }

        let mut processors = Vec::new();
        for i in 0..processor_count() {
            processors.push(Vcpu::new(i)?);
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
