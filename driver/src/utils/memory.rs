//! Everything related to memory.

use crate::utils::nt::{
    ExAllocatePool, ExFreePool, MmAllocateContiguousMemorySpecifyCacheNode, MmFreeContiguousMemory,
    MmIsAddressValid, MEMORY_CACHING_TYPE::MmCached, MM_ANY_NODE_OK,
};
use core::{
    ops::{Deref, DerefMut},
    ptr::NonNull,
};
use winapi::{
    km::wdm::POOL_TYPE::NonPagedPool, shared::ntdef::PHYSICAL_ADDRESS, um::winnt::RtlZeroMemory,
};
use x86::bits64::paging::{VAddr, BASE_PAGE_SIZE};

#[derive(Debug)]
pub enum AllocType {
    Normal,
    Contiguous,
}

/// Allocated memory that can never be null.
///
/// It will also automatically be deallocated when dropped. `Deref` and
/// `DerefMut` have been implemented to abstract the memory and actual code
/// behind it away. Because of this and generics, we can have any abstract data
/// allocated.
pub struct AllocatedMemory<T>(NonNull<T>, AllocType);

impl<T> AllocatedMemory<T> {
    /// Allocates executable non paged memory.
    pub fn alloc_executable(bytes: usize) -> Option<Self> {
        // NonPagedPoolExecutable = NonPagedPool
        Self::alloc(bytes)
    }

    /// Allocates normal non paged memory.
    pub fn alloc(bytes: usize) -> Option<Self> {
        let memory = unsafe { ExAllocatePool(NonPagedPool, bytes) };
        if memory.is_null() {
            log::warn!("Failed to allocate memory");
            return None;
        }

        // Zero the memory
        //
        unsafe { RtlZeroMemory(memory as _, bytes) };

        Some(Self(NonNull::new(memory as _)?, AllocType::Normal))
    }

    /// Allocates page aligned, zero filled physical memory.
    pub fn alloc_aligned(bytes: usize) -> Option<Self> {
        log::trace!("Allocating {} bytes of aligned physical memory", bytes);

        // The size must equal/greater than a page, to align it to a page
        //
        if bytes < BASE_PAGE_SIZE {
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
        let memory = Self(NonNull::new(memory as _)?, AllocType::Normal);

        // Make sure it's aligned
        //
        if !VAddr::from_u64(memory.as_ptr() as u64).is_base_page_aligned() {
            log::warn!("Memory is not aligned to a page");
            return None;
        }

        // Zero the memory
        //
        unsafe { RtlZeroMemory(memory.as_ptr() as _, bytes) };

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

        Some(Self(NonNull::new(memory as _)?, AllocType::Contiguous))
    }

    /// Frees the underlying memory.
    pub fn free(self) {
        core::mem::drop(self);
    }

    /// Checks whether the underlying memory buffer is null and whether the
    /// address pointing to it is valid.
    pub fn is_valid(&self) -> bool {
        unsafe { MmIsAddressValid(self.0.as_ref() as *const _ as _) }
    }

    pub const fn as_ptr(&self) -> *mut T {
        self.0.as_ptr()
    }

    pub const fn inner(&self) -> &NonNull<T> {
        &self.0
    }
}

impl<T> const AsRef<T> for AllocatedMemory<T> {
    fn as_ref(&self) -> &T {
        unsafe { self.0.as_ref() }
    }
}

impl<T> const AsMut<T> for AllocatedMemory<T> {
    fn as_mut(&mut self) -> &mut T {
        unsafe { self.0.as_mut() }
    }
}

impl<T> const Deref for AllocatedMemory<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe { self.0.as_ref() }
    }
}

impl<T> const DerefMut for AllocatedMemory<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { self.0.as_mut() }
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
            AllocType::Normal => {
                unsafe { ExFreePool(self.0.as_ptr() as _) };
            }
            AllocType::Contiguous => {
                unsafe { MmFreeContiguousMemory(self.0.as_ptr() as _) };
            }
        }
    }
}
