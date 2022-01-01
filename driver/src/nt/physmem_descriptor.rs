use crate::nt::include::{MmGetPhysicalMemoryRanges, PhysicalMemoryRange};
use crate::svm::paging::bytes_to_pages;
use core::fmt::{Debug, Formatter};

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
pub struct PhysicalMemoryDescriptor<'a> {
    pub number_of_runs: usize,
    pub number_of_pages: usize,

    // TODO: SimpleSvmHook stores the base_page and page_count instead of PhysicalMemoryRange
    pub ranges: &'a [PhysicalMemoryRange],
}

impl<'a> PhysicalMemoryDescriptor<'a> {
    pub fn new() -> Option<Self> {
        // See: https://doxygen.reactos.org/d1/d6d/dynamic_8c_source.html#l00073
        let memory_range = unsafe { MmGetPhysicalMemoryRanges() };
        if memory_range.is_null() {
            log::error!("MmGetPhysicalMemoryRanges() returned null");
            return None;
        }

        // Count the number of pages and runs
        //
        let mut number_of_runs = 0;
        let mut number_of_pages = 0;
        loop {
            let current = unsafe { memory_range.add(number_of_runs) };
            if current.is_null() {
                break;
            }

            let base_address = unsafe { (*current).base_address.QuadPart() };
            let number_of_bytes = unsafe { (*current).number_of_bytes.QuadPart() };
            if *base_address == 0 && *number_of_bytes == 0 {
                break;
            }

            log::trace!(
                "PhysicalMemoryDescriptor::new(): base_address={:#x}, number_of_bytes={:#x}",
                base_address,
                number_of_bytes
            );

            number_of_pages += bytes_to_pages!(number_of_bytes);
            number_of_runs += 1;
        }

        if number_of_runs == 0 {
            log::error!("PhysicalMemoryDescriptor::new(): no memory ranges found");
            return None;
        } else {
            Some(Self {
                number_of_runs,
                number_of_pages,
                ranges: unsafe { core::slice::from_raw_parts(memory_range, number_of_runs) },
            })
        }
    }
}

impl Debug for PhysicalMemoryDescriptor<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        log::info!("PhysicalMemoryDescriptor:");
        log::info!("  number_of_runs={}", self.number_of_runs);
        log::info!("  number_of_pages={}", self.number_of_pages);

        for range in self.ranges {
            let base_address = unsafe { (*range).base_address.QuadPart() };
            let number_of_bytes = unsafe { (*range).number_of_bytes.QuadPart() };

            f.write_fmt(format_args!(
                "  base_address = {:#x}, number_of_bytes: {:#x}, base_page: {:#x}, page_count: {:#x}\n",
                base_address, number_of_bytes, range.base_page(), range.page_count()
            ))?;
        }

        Ok(())
    }
}