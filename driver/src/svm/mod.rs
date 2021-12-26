extern crate alloc;

use crate::svm::data::shared_data::SharedData;

use x86::cpuid::{CpuId, Hypervisor};

pub mod data;
pub mod paging;
pub mod vmcb;
pub mod vmexit;

pub struct Processors {
    shard_data: SharedData,
    // processors: Vec<Processor>,
}

impl Processors {
    /// Creates new instance for all the processors on the system.
    ///
    /// # Assumptions
    ///
    /// The caller must already have checked whether the system supports virtualization.
    /// TODO: Should this be inside this instead? In terms of API usage, it would make sense to prevent mistakes.
    /// TODO: Return Result?
    pub fn new() -> Option<Self> {
        // let processors = (0..processor_count()).map(Processor::new).collect();

        Some(Self {
            shard_data: SharedData::new()?,
            // processors,
        })
    }

    pub fn virtualize(&self) -> bool {
        //
        false
    }
}

pub struct Processor {
    // The index of the processor.
    index: u32,
    data: (),
}

impl Processor {
    pub fn new(index: u32) -> Self {
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
}
