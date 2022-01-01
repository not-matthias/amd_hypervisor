use crate::nt::memory::AllocatedMemory;
use crate::svm::data::msr_bitmap::MsrBitmap;
use crate::svm::data::nested_page_table::NestedPageTable;

#[repr(C)]
pub struct SharedData {
    pub marker: u64,
    pub msr_permission_map: MsrBitmap,
    pub npt: AllocatedMemory<NestedPageTable>,
}

impl SharedData {
    pub fn new() -> Option<Self> {
        log::info!("Creating shared data");

        Some(Self {
            marker: 0x424242,
            msr_permission_map: MsrBitmap::new()?.build(),
            npt: NestedPageTable::identity()?,
        })
    }
}

impl Drop for SharedData {
    fn drop(&mut self) {
        log::info!("DROPPING SHARED DATA 1");
        log::info!("DROPPING SHARED DATA 2");
        log::info!("DROPPING SHARED DATA 3");
        log::info!("DROPPING SHARED DATA 4");
        log::info!("DROPPING SHARED DATA 5");
    }
}
