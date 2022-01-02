use crate::nt::memory::AllocatedMemory;
use crate::svm::data::msr_bitmap::MsrBitmap;
use crate::svm::data::nested_page_table::NestedPageTable;

#[repr(C)]
pub struct SharedData {
    pub msr_permission_map: MsrBitmap,
    pub npt: AllocatedMemory<NestedPageTable>,
}

impl SharedData {
    pub fn new() -> Option<AllocatedMemory<Self>> {
        log::info!("Creating shared data");

        let mut data = AllocatedMemory::<Self>::alloc(core::mem::size_of::<Self>())?;

        // This is safe because the memory can never be null.
        unsafe {
            (*data.ptr()).msr_permission_map = MsrBitmap::new()?.build();
            (*data.ptr()).npt = NestedPageTable::identity()?;
        }

        Some(data)
    }
}
