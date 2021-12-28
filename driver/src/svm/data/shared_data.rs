use crate::svm::data::msr_bitmap::MsrBitmap;
use crate::svm::data::nested_page_table::NestedPageTableWrapper;

pub struct SharedData {
    pub msr_permission_map: MsrBitmap,
    pub npt: NestedPageTableWrapper,
}

impl SharedData {
    pub fn new() -> Option<Self> {
        log::info!("Creating shared data");

        Some(Self {
            msr_permission_map: MsrBitmap::new()?.build(),
            npt: unsafe { NestedPageTableWrapper::new()?.build() },
        })
    }
}
