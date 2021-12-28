use crate::nt::memory::AllocatedMemory;
use crate::svm::data::msr_bitmap::MsrBitmap;
use crate::svm::data::nested_page_table::NestedPageTable;

pub struct SharedData {
    pub msr_permission_map: AllocatedMemory<MsrBitmap>,
    pub npt: AllocatedMemory<NestedPageTable>,
}

impl SharedData {
    pub fn new() -> Option<Self> {
        log::info!("Creating shared data");

        Some(Self {
            msr_permission_map: MsrBitmap::new()?.build(),
            npt: unsafe { NestedPageTable::new()?.build() },
        })
    }
}
