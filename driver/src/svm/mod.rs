extern crate alloc;

use crate::nt::processor::processor_count;
use crate::support;
use crate::svm::data::shared_data::SharedData;
use alloc::vec::Vec;
use x86::cpuid::{CpuId, Hypervisor};

pub mod data;
pub mod paging;
pub mod vmcb;
pub mod vmexit;

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
            processors: (0..processor_count()).map(Processor::new).collect(),
        })
    }

    pub fn virtualize(&self) -> bool {
        for processor in &self.processors {
            if !processor.virtualize() {
                log::error!("Failed to virtualize processor {}", processor.id());
                return false;
            }
        }

        true
    }
}

pub struct Processor {
    // The index of the processor.
    index: u32,
    data: (),
}

impl Processor {
    pub fn new(index: u32) -> Self {
        log::trace!("Creating processor {}", index);

        // TODO:
        // - Allocate context
        // - Allocate per processor data (VIRTUAL_PROCESSOR_DATA)
        //   - GuestVmcb
        //   - HostVmcb
        //   - Stack, TrapFrame (?)

        Self { index, data: () }
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

    pub fn virtualize(&self) -> bool {
        //
        false
    }

    fn launch_vm(&self) {
        // https://github.com/tandasat/SimpleSvm/blob/master/SimpleSvm/x64.asm#L78
        //
    }

    pub fn devirtualize(&self) {
        // TODO: Call cpuid with custom parameters
    }

    pub fn id(&self) -> u32 {
        self.index
    }
}
