//! Everything related to memory.

use crate::nt::include::{
    ExAllocatePool, ExFreePool, MmAllocateContiguousMemorySpecifyCacheNode, MmFreeContiguousMemory,
    MEMORY_CACHING_TYPE::MmCached, MM_ANY_NODE_OK,
};
use core::ops::{Deref, DerefMut};
use winapi::shared::ntdef::PVOID;
use winapi::um::winnt::RtlZeroMemory;
use winapi::{km::wdm::POOL_TYPE::NonPagedPool, shared::ntdef::PHYSICAL_ADDRESS};

// TODO: Move to paging module
pub const PAGE_SHIFT: u64 = 12;
pub const PAGE_SIZE: usize = 0x1000;
const PAGE_MASK: usize = !(PAGE_SIZE - 1);

/// Aligns the specified virtual address to a page.
///
/// # Example
/// ```
/// let page = page_align!(4097);
/// assert_eq!(page, 4096);
/// ```
///
/// # Credits
/// // See: https://stackoverflow.com/questions/20771394/how-to-understand-the-macro-of-page-align-in-kernel/20771666
#[macro_export]
macro_rules! page_align {
    ($virtual_address:expr) => {
        ($virtual_address + PAGE_SIZE - 1) & PAGE_MASK
    };
}

pub struct AlignedMemory<T>(*mut T);

impl<T> AlignedMemory<T> {
    /// Allocates page aligned, zero filled physical memory.
    pub fn alloc(bytes: usize) -> Option<Self> {
        log::trace!("Allocating {} bytes of aligned physical memory", bytes);

        // The size must equal/greater than a page, to align it to a page
        //
        if bytes < PAGE_SIZE {
            log::warn!("Allocating memory failed: size is smaller than a page");
            return None;
        }

        // Allocate memory
        //
        let memory = unsafe { ExAllocatePool(NonPagedPool, bytes) };
        if memory.is_null() {
            log::warn!("Failed to allocate memory");
            return None;
        }

        // Make sure it's aligned
        //
        if page_align!(memory as usize) != memory as usize {
            log::warn!("Memory is not aligned to a page");
            return None;
        }

        // Zero the memory
        //
        unsafe { RtlZeroMemory(memory, bytes) };

        Some(Self(memory as *mut T))
    }

    /// Frees the underlying memory.
    pub fn free(self) {
        unsafe { ExFreePool(self.0 as _) };
    }
}

impl<T> Deref for AlignedMemory<T> {
    type Target = *mut T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> DerefMut for AlignedMemory<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<T> Drop for AlignedMemory<T> {
    fn drop(&mut self) {
        // TODO: Can we somehow capture self here?

        log::trace!("Freeing aligned physical memory");
        unsafe { ExFreePool(self.0 as _) };
    }
}

/// Allocates page aligned, zero filled contiguous physical memory.
///
/// # What is contiguous memory?
/// Click [here](https://stackoverflow.com/questions/4059363/what-is-a-contiguous-memory-block).
pub fn alloc_contiguous(bytes: usize) -> Option<*mut u64> {
    log::trace!("Allocating {} bytes of contiguous physical memory", bytes);

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

    Some(memory as *mut u64)
}

/// Frees the previously allocated contiguous memory.
pub fn free_contiguous(address: *mut u64) {
    unsafe { MmFreeContiguousMemory(address as _) }
}
