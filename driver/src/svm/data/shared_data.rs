use crate::hook::HookedNpt;
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
        data.hooked_npt.hook("ZwQuerySystemInformation", 0 as _)?;

        // data.hooked_npt = NestedPageTable::identity_2mb()?;

        Some(data)
    }
}
