use crate::nt::include::{RtlClearAllBits, RtlInitializeBitMap, RtlSetBits, RTL_BITMAP};
use crate::nt::memory::{alloc_aligned, alloc_contiguous, PAGE_SIZE};
use crate::svm::data::msr_bitmap::MsrBitmap;
use crate::svm::data::nested_page_table::NestedPageTable;
use nt::include::PVOID;
use x86::bits64::paging::PML4Entry;
use x86::bits64::paging::{PDEntry, PML4};

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
