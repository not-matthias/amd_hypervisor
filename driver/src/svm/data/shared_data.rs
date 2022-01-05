extern crate alloc;

use crate::hook::{Hook, HookedNpt};
use crate::nt::memory::AllocatedMemory;
use crate::svm::data::msr_bitmap::MsrBitmap;
use alloc::vec::Vec;

#[repr(C)]
pub struct SharedData {
    pub msr_permission_map: AllocatedMemory<u32>,
    pub hooked_npt: AllocatedMemory<HookedNpt>,
}

impl SharedData {
    pub fn new(hooks: Vec<Hook>) -> Option<AllocatedMemory<Self>> {
        log::info!("Creating shared data");

        let mut data = AllocatedMemory::<Self>::alloc(core::mem::size_of::<Self>())?;
        data.msr_permission_map = MsrBitmap::new()?;
        data.hooked_npt = HookedNpt::new(hooks)?;

        Some(data)
    }
}
