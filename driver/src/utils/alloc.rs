use crate::utils::nt::{ExAllocatePoolWithTag, ExFreePool};
use core::alloc::{GlobalAlloc, Layout};
use winapi::km::wdm::POOL_TYPE;

/// 'tsuR'
const TAG: u32 = 0x7473_7552;

/// The global kernel allocator.
pub struct KernelAlloc;

unsafe impl GlobalAlloc for KernelAlloc {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let pool =
            ExAllocatePoolWithTag(POOL_TYPE::NonPagedPool as u32, layout.size(), TAG) as *mut u64;

        if pool.is_null() {
            #[cfg(not(feature = "no-assertions"))]
            panic!("Allocation failed.");

            #[cfg(feature = "no-assertions")]
            panic!()
        }

        pool as _
    }

    unsafe fn dealloc(&self, ptr: *mut u8, _layout: Layout) {
        ExFreePool(ptr as _);
    }
}

#[cfg_attr(not(test), alloc_error_handler)]
fn alloc_error(layout: Layout) -> ! {
    #[cfg(not(feature = "no-assertions"))]
    panic!("{:?} alloc memory error", layout);

    #[cfg(feature = "no-assertions")]
    panic!()
}
