//! Everything related to memory.

use crate::nt::include::{
    ExAllocatePool, ExFreePool, MmAllocateContiguousMemorySpecifyCacheNode, MmFreeContiguousMemory,
    MEMORY_CACHING_TYPE::MmCached, MM_ANY_NODE_OK,
};
use crate::svm::paging::{page_align, PAGE_SIZE};
use core::ops::{Deref, DerefMut};
use nt::include::MmIsAddressValid;
use winapi::um::winnt::RtlZeroMemory;
use winapi::{km::wdm::POOL_TYPE::NonPagedPool, shared::ntdef::PHYSICAL_ADDRESS};

#[derive(Debug)]
#[repr(C)]
pub enum AllocType {
    Aligned,
    Contiguous,
}

/// Allocated memory that can never be null.
///
/// It will also automatically be deallocated when dropped. `Deref` and `DerefMut` have been
/// implemented to abstract the memory and actual code behind it away. Because of this and
/// generics, we can have any abstract data allocated.
///
#[repr(C)]
pub struct AllocatedMemory<T>(*mut T, AllocType);

impl<T> AllocatedMemory<T> {
    /// Allocates page aligned, zero filled physical memory.
    pub fn alloc_aligned(bytes: usize) -> Option<Self> {
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
        let mut memory = Self(memory as *mut T, AllocType::Aligned);

        // Make sure it's aligned
        //
        if page_align!(memory.0 as usize) != memory.0 as usize {
            log::warn!("Memory is not aligned to a page");
            return None;
        }

        // Zero the memory
        //
        unsafe { RtlZeroMemory(memory.ptr() as _, bytes) };

        Some(memory)
    }

    /// Allocates page aligned, zero filled contiguous physical memory.
    ///
    /// # What is contiguous memory?
    /// Click [here](https://stackoverflow.com/questions/4059363/what-is-a-contiguous-memory-block).
    pub fn alloc_contiguous(bytes: usize) -> Option<Self> {
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

        Some(Self(memory as *mut T, AllocType::Contiguous))
    }

    /// Frees the underlying memory.
    pub fn free(self) {
        core::mem::drop(self);
    }

    /// Returns a pointer to the underlying memory.
    pub const fn ptr(&mut self) -> *mut T {
        self.0
    }

    /// Checks whether the underlying memory buffer is null and whether the address pointing to it is valid.
    pub fn is_valid(&mut self) -> bool {
        !self.ptr().is_null() && unsafe { MmIsAddressValid(self.ptr() as _) }
    }
}

impl<T> Deref for AllocatedMemory<T> {
    type Target = *mut T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> DerefMut for AllocatedMemory<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<T> Drop for AllocatedMemory<T> {
    fn drop(&mut self) {
        if !self.is_valid() {
            log::trace!("Pointer is not valid. Already deallocated?");
            return;
        }

        log::trace!("Freeing physical memory: {:p} - {:?}", self.0, self.1);

        match self.1 {
            AllocType::Aligned => {
                unsafe { ExFreePool(self.0 as _) };
            }
            AllocType::Contiguous => {
                unsafe { MmFreeContiguousMemory(self.0 as _) };
            }
        }
    }
}
