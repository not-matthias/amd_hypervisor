use crate::nt::include::{ExFreePool, MmGetPhysicalMemoryRanges};
use crate::svm::paging::{bytes_to_pages, _1GB};
use core::fmt::Debug;
use tinyvec::ArrayVec;
use x86::bits32::paging::BASE_PAGE_SIZE;
use x86::bits64::paging::BASE_PAGE_SHIFT;

/// https://github.com/wbenny/hvpp/blob/84b3f3c241e1eec3ab42f75cad9deef3ad67e6ab/src/hvpp/hvpp/lib/mm/physical_memory_descriptor.h#L22
///
/// TODO: Find out why this is the constant we need?
const MAX_RANGE_COUNT: usize = 32;

#[derive(Debug, Default)]
pub struct PhysicalMemoryRange {
    pub base_address: u64,
    pub number_of_bytes: u64,
}

impl PhysicalMemoryRange {
    pub fn base_page(&self) -> u64 {
        self.base_address >> BASE_PAGE_SHIFT
    }

    pub fn page_count(&self) -> u64 {
        bytes_to_pages!(self.number_of_bytes) as u64
    }
}

///
///
/// You can use `RAMMap` to verify and see the physical memory ranges of your system: https://codemachine.com/articles/physical_memory_ranges_in_kernel_debugger.html
///
/// ## What is this and why are there multiple physical memory ranges?
///
/// This is due to different memory mappings. You can't change them because they are hardware mappings,
/// which leaves holes in the physical memory address space.
///
/// For more information, see the OSDev wiki: https://wiki.osdev.org/Memory_Map_(x86)
///
/// Thanks for @PDBDream
///
#[derive(Debug)]
pub struct PhysicalMemoryDescriptor {
    ranges: ArrayVec<[PhysicalMemoryRange; MAX_RANGE_COUNT]>,
    count: usize,
}

impl PhysicalMemoryDescriptor {
    pub fn new() -> Self {
        // See: https://doxygen.reactos.org/d1/d6d/dynamic_8c_source.html#l00073
        //
        let memory_range = unsafe { MmGetPhysicalMemoryRanges() };
        if memory_range.is_null() {
            log::error!("MmGetPhysicalMemoryRanges() returned null");
            unreachable!()
        }

        // Count the number of pages and runs
        //
        let mut ranges = ArrayVec::new();

        let mut count = 0;
        loop {
            let current = unsafe { memory_range.add(count) };
            if current.is_null() {
                break;
            }

            let base_address = unsafe { *(*current).base_address.QuadPart() as u64 };
            let number_of_bytes = unsafe { *(*current).number_of_bytes.QuadPart() as u64 };
            if base_address == 0 && number_of_bytes == 0 {
                break;
            }

            ranges.push(PhysicalMemoryRange {
                base_address,
                number_of_bytes,
            });

            count += 1;
        }

        unsafe { ExFreePool(memory_range as *mut _) };

        if count == 0 {
            log::error!("PhysicalMemoryDescriptor::new(): no memory ranges found");
            unreachable!()
        }

        Self { ranges, count }
    }

    pub fn get_ranges(&self) -> &[PhysicalMemoryRange] {
        &self.ranges[0..self.count]
    }

    /// Returns the number of physical memory pages.
    pub fn page_count(&self) -> usize {
        self.get_ranges()
            .iter()
            .fold(0, |acc, range| acc + bytes_to_pages!(range.number_of_bytes))
    }

    /// Returns the total physical memory size in bytes.
    pub fn total_size(&self) -> usize {
        self.page_count() * BASE_PAGE_SIZE
    }

    /// Returns the total physical memory size in giga bytes.
    pub fn total_size_in_gb(&self) -> usize {
        self.total_size() / _1GB as usize
    }
}
