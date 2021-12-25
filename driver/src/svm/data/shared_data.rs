use crate::svm::data::msr_bitmap::MsrBitmap;
use crate::svm::data::nested_page_table::NestedPageTable;

#[repr(C, align(4096))]
pub struct SharedData {
    pub msr_permission_map: MsrBitmap,
    pub npt: NestedPageTable,
}

impl SharedData {
    pub fn new() -> Option<Self> {
        Some(Self {
            msr_permission_map: MsrBitmap::new()?.build(),
            npt: unsafe { NestedPageTable::new()?.build() },
        })
    }
}
