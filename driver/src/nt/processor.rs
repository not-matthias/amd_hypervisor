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

/// Switches the execution of a process to another.
fn switch_execution(
    processor_number: PROCESSOR_NUMBER,
    mut old_affinity: GROUP_AFFINITY,
) -> GROUP_AFFINITY {
    let mut affinity: GROUP_AFFINITY = unsafe { core::mem::zeroed() };

    affinity.Group = processor_number.Group;
    affinity.Mask = 1 << processor_number.Number;
    affinity.Reserved[0] = 0;
    affinity.Reserved[1] = 0;
    affinity.Reserved[2] = 0;

    unsafe { KeSetSystemGroupAffinityThread(&mut affinity, &mut old_affinity) };

    affinity
}

/// Executes the specified function on a specific processor.
pub fn execute_on_processor<F, D>(i: u32, f: &F, data: D) -> Option<()>
where
    F: Fn(D) -> Option<()>,
{
    if i > processor_count() {
        return None;
    }

    let processor_number = processor_number_from_index(i)?;

    // Switch execution of this code to a processor #i.
    //
    let mut old_affinity = switch_execution(processor_number, unsafe { core::mem::zeroed() });

    // Execute the callback
    //
    let status = f(data);

    // Revert the previously executed processor.
    //
    unsafe { KeRevertToUserGroupAffinityThread(&mut old_affinity) };

    status
}

/// Executes the specified function on each processor.
pub fn execute_on_each_processor<F, D>(f: F, data: &D) -> Option<()>
where
    F: Fn(&D) -> Option<()>,
{
    for i in 0..processor_count() {
        execute_on_processor(i, &f, data)?;
    }

    Some(())
}
