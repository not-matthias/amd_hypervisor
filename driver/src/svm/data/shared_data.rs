use crate::hook::{Hook, HookedNpt};
use crate::nt::memory::AllocatedMemory;
use crate::svm::data::msr_bitmap::MsrBitmap;

#[repr(C)]
pub struct SharedData {
    pub msr_permission_map: AllocatedMemory<u32>,
    pub hooked_npt: AllocatedMemory<HookedNpt>,
}

impl SharedData {
    pub fn new() -> Option<AllocatedMemory<Self>> {
        log::info!("Creating shared data");

        let mut data = AllocatedMemory::<Self>::alloc(core::mem::size_of::<Self>())?;
        data.msr_permission_map = MsrBitmap::new()?;
        data.hooked_npt = HookedNpt::new()?;
        // data.hooked_npt.hook("ZwQuerySystemInformation", 0 as _)?;

        // Create the hook and change the 3rd byte (return value)
        //

        let hook = unsafe {
            Hook::from_address(crate::hook_testing::ALLOCATED_MEMORY.as_ref()?.as_ptr() as _)?
        };
        data.hooked_npt.hooks.push(hook);

        // data.hooked_npt = NestedPageTable::identity_2mb()?;

        Some(data)
    }
}
