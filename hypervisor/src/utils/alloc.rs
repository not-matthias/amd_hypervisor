use crate::utils::nt::{
    ExAllocatePoolWithTag, ExFreePool, MmAllocateContiguousMemorySpecifyCacheNode,
    MmFreeContiguousMemory, MEMORY_CACHING_TYPE::MmCached, MM_ANY_NODE_OK,
};
use core::{
    alloc::{AllocError, Allocator, GlobalAlloc, Layout},
    ptr::NonNull,
};
use winapi::{km::wdm::POOL_TYPE, shared::ntdef::PHYSICAL_ADDRESS};

/// 'tsuR'
const TAG: u32 = 0x7473_7552;

/// Allocates **contiguous** physical memory.
pub struct PhysicalAllocator;

unsafe impl Allocator for PhysicalAllocator {
    fn allocate(&self, layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
        let mut boundary: PHYSICAL_ADDRESS = unsafe { core::mem::zeroed() };
        let mut lowest: PHYSICAL_ADDRESS = unsafe { core::mem::zeroed() };
        let mut highest: PHYSICAL_ADDRESS = unsafe { core::mem::zeroed() };

        unsafe { *(boundary.QuadPart_mut()) = 0 };
        unsafe { *(lowest.QuadPart_mut()) = 0 };
        unsafe { *(highest.QuadPart_mut()) = -1 };

        let memory = unsafe {
            MmAllocateContiguousMemorySpecifyCacheNode(
                layout.size(),
                lowest,
                highest,
                boundary,
                MmCached,
                MM_ANY_NODE_OK,
            )
        } as *mut u8;
        if memory.is_null() {
            Err(AllocError)
        } else {
            let slice = unsafe { core::slice::from_raw_parts_mut(memory, layout.size()) };
            Ok(unsafe { NonNull::new_unchecked(slice) })
        }
    }

    unsafe fn deallocate(&self, ptr: NonNull<u8>, _layout: Layout) {
        MmFreeContiguousMemory(ptr.cast().as_ptr());
    }
}

/// Allocates non-paged virtual memory.
#[cfg(feature = "allocator")]
pub struct KernelAlloc;

#[cfg(feature = "allocator")]
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

#[cfg_attr(feature = "allocator", alloc_error_handler)]
pub fn alloc_error(layout: Layout) -> ! {
    panic!("{:?} alloc memory error", layout);
}
