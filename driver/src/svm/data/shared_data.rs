use crate::nt::memory::AllocatedMemory;
use crate::svm::data::msr_bitmap::MsrBitmap;
use crate::svm::data::nested_page_table::NestedPageTable;

#[repr(C)]
pub struct SharedData {
    pub msr_permission_map: AllocatedMemory<u32>,
    pub npt: AllocatedMemory<NestedPageTable>,
}

impl SharedData {
    pub fn new() -> Option<AllocatedMemory<Self>> {
        log::info!("Creating shared data");

        let mut data = AllocatedMemory::<Self>::alloc(core::mem::size_of::<Self>())?;
        data.msr_permission_map = MsrBitmap::new()?;
        data.npt = NestedPageTable::identity_new()?;

        Some(data)
    }
}
