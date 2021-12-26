extern crate alloc;

use crate::nt::processor::processor_count;
use crate::support;

use crate::svm::data::processor::ProcessorDataWrapper;
use crate::svm::data::shared_data::SharedData;
use alloc::vec::Vec;

use crate::svm::data::msr_bitmap::EFER_SVME;
use x86::cpuid::{CpuId, Hypervisor};
use x86::msr::{rdmsr, wrmsr, IA32_EFER};

pub mod data;
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

        Some(Self {
            shard_data: SharedData::new()?,
            processors: (0..processor_count()).filter_map(Processor::new).collect(),
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
}

pub struct Processor {
    /// The index of the processor.
    index: u32,

    data: ProcessorDataWrapper,
}

impl Processor {
    pub fn new(index: u32) -> Option<Self> {
        log::trace!("Creating processor {}", index);

        // TODO:
        // - Allocate context
        // - Allocate per processor data (VIRTUAL_PROCESSOR_DATA)
        //   - GuestVmcb
        //   - HostVmcb
        //   - Stack, TrapFrame (?)

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

                    // TODO: Only allow our hypervisor

                    true
                }
                _ => false,
            })
            .unwrap_or_default()
    }

    pub fn launch_vm(&self) {
        // https://github.com/tandasat/SimpleSvm/blob/master/SimpleSvm/x64.asm#L78
        //

        let host_rsp = self.data.data.host_stack_layout.guest_vmcb_pa;
    }

    pub fn virtualize(&mut self, shared_data: &SharedData) -> bool {
        // Based on this: https://github.com/tandasat/SimpleSvm/blob/master/SimpleSvm/SimpleSvm.cpp#L1137

        // Check if already virtualized.
        //
        if self.is_virtualized() {
            log::info!("Processor {} is already virtualized", self.index);
            return true;
        }

        // Attempt to virtualize the processor
        //

        // Enable SVM by setting EFER.SVME.
        let msr = unsafe { rdmsr(IA32_EFER) } | EFER_SVME;
        unsafe { wrmsr(IA32_EFER, msr) };

        // Setup vmcb
        //
        self.data.prepare_for_virtualization(shared_data);

        // Launch vm
        //
        self.launch_vm();

        log::info!("We should have never been here.");
        true
    }

    pub fn devirtualize(&self) {
        // TODO: Call cpuid with custom parameters
    }

    pub fn id(&self) -> u32 {
        self.index
    }
}
