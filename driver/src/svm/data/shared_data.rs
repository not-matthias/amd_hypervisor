extern crate alloc;

use crate::hook::npt::DuplicateNptHook;
use crate::nt::memory::AllocatedMemory;
use crate::svm::data::msr_bitmap::MsrBitmap;
use crate::Hook;
use alloc::vec::Vec;

#[repr(C)]
pub struct SharedData {
    pub msr_permission_map: AllocatedMemory<u32>,
    pub hooked_npt: AllocatedMemory<DuplicateNptHook>,
}

impl SharedData {
    pub fn new(hooks: Vec<Hook>) -> Option<AllocatedMemory<Self>> {
        log::info!("Creating shared data");

        let mut data = AllocatedMemory::<Self>::alloc(core::mem::size_of::<Self>())?;
        data.msr_permission_map = MsrBitmap::new()?;
        data.hooked_npt = DuplicateNptHook::new(hooks)?;

        Some(data)
    }
}
