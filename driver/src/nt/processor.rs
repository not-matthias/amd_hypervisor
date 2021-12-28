//! Handles everything related to the physical processors.

use crate::nt::include::{
    KeGetProcessorNumberFromIndex, KeQueryActiveProcessorCountEx,
    KeRevertToUserGroupAffinityThread, KeSetSystemGroupAffinityThread,
};
use core::mem::MaybeUninit;
use winapi::shared::ntdef::{ALL_PROCESSOR_GROUPS, GROUP_AFFINITY, NT_SUCCESS, PROCESSOR_NUMBER};

pub fn processor_count() -> u32 {
    unsafe { KeQueryActiveProcessorCountEx(ALL_PROCESSOR_GROUPS) }
}

/// Returns the processor number for the specified index.
fn processor_number_from_index(index: u32) -> Option<PROCESSOR_NUMBER> {
    let mut processor_number = MaybeUninit::uninit();

    let status = unsafe { KeGetProcessorNumberFromIndex(index, processor_number.as_mut_ptr()) };
    if !NT_SUCCESS(status) {
        None
    } else {
        Some(unsafe { processor_number.assume_init() })
    }
}

/// Switches execution to a specific processor until dropped.
pub struct ProcessorExecutor {
    old_affinity: MaybeUninit<GROUP_AFFINITY>,
}

impl ProcessorExecutor {
    pub fn switch_to_processor(i: u32) -> Option<Self> {
        if i > processor_count() {
            log::error!("Invalid processor index: {}", i);
            return None;
        }

        let processor_number = processor_number_from_index(i)?;

        let mut old_affinity = MaybeUninit::uninit();
        let mut affinity: GROUP_AFFINITY = unsafe { core::mem::zeroed() };

        affinity.Group = processor_number.Group;
        affinity.Mask = 1 << processor_number.Number;
        affinity.Reserved[0] = 0;
        affinity.Reserved[1] = 0;
        affinity.Reserved[2] = 0;

        log::trace!("Switching execution to processor {}", i);
        unsafe { KeSetSystemGroupAffinityThread(&mut affinity, old_affinity.as_mut_ptr()) };

        Some(Self { old_affinity })
    }
}

impl Drop for ProcessorExecutor {
    fn drop(&mut self) {
        log::info!("Switching execution back to previous processor");
        unsafe {
            KeRevertToUserGroupAffinityThread(self.old_affinity.as_mut_ptr());
        }
    }
}
