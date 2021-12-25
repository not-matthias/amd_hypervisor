//! Everything related to memory.

use crate::nt::include::{
    ExAllocatePool, ExFreePool, MmAllocateContiguousMemorySpecifyCacheNode, MmFreeContiguousMemory,
    MEMORY_CACHING_TYPE::MmCached, MM_ANY_NODE_OK,
};
use winapi::shared::ntdef::PVOID;
use winapi::um::winnt::RtlZeroMemory;
use winapi::{km::wdm::POOL_TYPE::NonPagedPool, shared::ntdef::PHYSICAL_ADDRESS};

// TODO: Move to paging module
pub const PAGE_SHIFT: u64 = 12;
pub const PAGE_SIZE: usize = 0x1000;

/// Aligns the specified virtual address to a page.
///
/// # Example
/// ```
/// let page = page_align!(4097);
/// assert_eq!(page, 4096);
/// ```
#[macro_export]
macro_rules! page_align {
    ($virtual_address:expr) => {
        $virtual_address & (PAGE_SIZE - 1)
    };
}

/// Allocates page aligned, zero filled physical memory.
pub fn alloc_aligned(bytes: usize) -> Option<PVOID> {
    // The size must equal/greater than a page, to align it to a page
    //
    if bytes < PAGE_SIZE {
        return None;
    }

    // Allocate memory
    //
    let memory = unsafe { ExAllocatePool(NonPagedPool, bytes) };
    if memory.is_null() {
        return None;
    }

    // Make sure it's aligned
    //
    if page_align!(memory as usize) != memory as usize {
        return None;
    }

    // Zero the memory
    //
    unsafe { RtlZeroMemory(memory, bytes) };

    Some(memory)
}

/// Frees the allocated memory.
pub fn free_aligned(address: PVOID) {
    unsafe { ExFreePool(address) }
}

/// Allocates page aligned, zero filled contiguous physical memory.
///
/// # What is contiguous memory?
/// Click [here](https://stackoverflow.com/questions/4059363/what-is-a-contiguous-memory-block).
pub fn alloc_contiguous(bytes: usize) -> Option<PVOID> {
    let mut boundary: PHYSICAL_ADDRESS = unsafe { core::mem::zeroed() };
    let mut lowest: PHYSICAL_ADDRESS = unsafe { core::mem::zeroed() };
    let mut highest: PHYSICAL_ADDRESS = unsafe { core::mem::zeroed() };

    unsafe { *(boundary.QuadPart_mut()) = 0 };
    unsafe { *(lowest.QuadPart_mut()) = 0 };
    unsafe { *(highest.QuadPart_mut()) = -1 };

    let memory = unsafe {
        MmAllocateContiguousMemorySpecifyCacheNode(
            bytes,
            lowest,
            highest,
            boundary,
            MmCached,
            MM_ANY_NODE_OK,
        )
    };

    // Return `None` if the memory is null
    //
    if memory.is_null() {
        return None;
    }

    // Zero the memory
    //
    unsafe { RtlZeroMemory(memory, bytes) };

    Some(memory)
}

/// Frees the previously allocated contiguous memory.
pub fn free_contiguous(address: PVOID) {
    unsafe { MmFreeContiguousMemory(address) }
}
