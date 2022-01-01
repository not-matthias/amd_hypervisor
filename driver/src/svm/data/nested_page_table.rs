use crate::nt::addresses::physical_address;
use crate::nt::memory::AllocatedMemory;
use crate::svm::paging::{pfn_from_pa, va_from_pfn, PFN_MASK};
use crate::PhysicalMemoryDescriptor;
use elain::Align;
use x86::bits64::paging::{
    pd_index, pdpt_index, pml4_index, pt_index, PAddr, PDEntry, PDFlags, PDPTEntry, PDPTFlags,
    PML4Entry, PML4Flags, VAddr, BASE_PAGE_SIZE, PAGE_SIZE_ENTRIES, PD, PDPT, PML4,
};

#[repr(C, align(4096))]
pub struct NestedPageTable {
    pub pml4: PML4,
    align_0: Align<4096>,

    pub pdp_entries: PDPT,
    align_1: Align<4096>,

    pub pd_entries: [PD; 512],
    align_2: Align<4096>,
}

impl NestedPageTable {
    /// Creates the 2MB identity page table. Maps every guest physical address to the same host
    /// physical address.
    /// This means physical address 0x4000 in the guest will point to the physical memory 0x4000 in the host.
    ///
    ///
    /// # How it works
    ///
    /// We create a page table with **2MB** instead of **4KB** pages. There's multiple reasons for that:
    /// - Smaller page table.
    /// - Iterating is faster since we remove 1 iteration.
    ///
    /// Pros:
    /// - Easier to implement.
    /// - Faster to iterate (3 levels instead of 4).
    ///
    /// Cons:
    /// - We probably don't need access to 512 GB of physical memory.
    /// - Hooking 2MB pages is inconvenient, because we would get tons of ept violations.
    ///
    /// # Other implementations
    ///
    /// Even though other hypervisors might be built for Intel processors, they still need to build
    /// some kind of [SLAT](https://en.wikipedia.org/wiki/Second_Level_Address_Translation) (Second Level Address Translation Table).
    ///
    /// Here's a list of useful references in popular projects:
    /// - [hvpp](https://github.com/wbenny/hvpp/blob/master/src/hvpp/hvpp/ept.cpp#L41)
    /// - [gbhv](https://github.com/Gbps/gbhv/blob/master/gbhv/ept.c#L167)
    ///
    pub fn identity() -> Option<AllocatedMemory<Self>> {
        log::info!("Building nested page tables");

        let mut npt =
            AllocatedMemory::<Self>::alloc_aligned(core::mem::size_of::<NestedPageTable>())?;

        // PML4
        //
        unsafe {
            (**npt).pml4[0] = PML4Entry::new(
                physical_address((**npt).pdp_entries.as_ptr() as _),
                PML4Flags::from_iter([PML4Flags::P, PML4Flags::RW, PML4Flags::US]),
            )
        };

        // PDPT
        //
        for (i, pdp) in unsafe { (**npt).pdp_entries }.iter_mut().enumerate() {
            // for (i, pdp) in unsafe { (**npt).pdp_entries.iter_mut().enumerate() } {
            let pa = physical_address(unsafe { (**npt).pd_entries[i].as_ptr() as _ });
            *pdp = PDPTEntry::new(
                pa,
                PDPTFlags::from_iter([PDPTFlags::P, PDPTFlags::RW, PDPTFlags::US]),
            );

            // PD
            //
            for (j, pd) in unsafe { (**npt).pd_entries[i].iter_mut().enumerate() } {
                let pa = (i * PAGE_SIZE_ENTRIES + j) as u64;

                // Mask to find the page frame number. We have to use this so that we can use `PDEntry`.
                // This is the same as using a `bitflags` struct and setting the bits `21-51`.
                //
                let pfn = pa << 21 & PFN_MASK;

                *pd = PDEntry::new(
                    PAddr::from(pfn),
                    PDFlags::from_iter([PDFlags::P, PDFlags::RW, PDFlags::US, PDFlags::PS]),
                );
            }
        }

        Some(npt)
    }

    /// Builds the nested page table to cover for the entire physical memory address space.
    ///
    ///
    pub fn system() -> Option<()> {
        let desc = PhysicalMemoryDescriptor::new()?;
        let npt = AllocatedMemory::<Self>::alloc_aligned(core::mem::size_of::<NestedPageTable>())?;

        // NPT entries based on the physical memory ranges.
        //
        for range in desc.ranges {
            let base_address = range.base_page() * BASE_PAGE_SIZE as u64;

            for page_index in 0..range.page_count() {
                let indexed_address = base_address + page_index * BASE_PAGE_SIZE as u64;

                let entry = npt.build_sub_tables(indexed_address);
                // TODO: Check if entry is valid
                //
            }
        }

        // Entry for APIC base
        //
        // apicBase.AsUInt64 = __readmsr(IA32_APIC_BASE);
        // entry = BuildSubTables(pml4Table, apicBase.Fields.ApicBase * PAGE_SIZE, FALSE);
        // if (entry == nullptr) {
        //     status = STATUS_INSUFFICIENT_RESOURCES;
        //     goto Exit;
        // }

        // Compute max PDPT index based on last descriptor entry that describes the highest pa.
        //
        let last_range = desc.ranges.last().unwrap();
        let base_address = last_range.base_page() * BASE_PAGE_SIZE as u64;
        // let _max_pdp_index =
        //     (base_address + last_range.page_count() * BASE_PAGE_SIZE as u64).pdp_index();
        // TODO: ROUND_TO_SIZE
        // maxPpeIndex = ROUND_TO_SIZE(baseAddr + currentRun->PageCount * PAGE_SIZE,
        //                             oneGigabyte) / oneGigabyte;

        Some(())
    }

    fn build_sub_tables(self: &AllocatedMemory<Self>, physical_address: u64) -> () {
        let physical_address = VAddr::from(physical_address);

        // TODO: NptOperation -> FindOperation or BuildOperation

        // PML4 (512 GB)
        //
        let pml4_index = pml4_index(physical_address);
        log::info!("PML4 index: {}", pml4_index);

        // let pml4_entry = (*self).pml4[pml4_index];
        // if !pml4_entry.is_present() {
        //     // if !build_npt_entry(pml4_entry, u64::MAX) { exit }
        // }
        // // TODO: Validate that this is the page frame number
        // let pdpt = va_from_pfn!(pml4_entry.address().as_u64());
        // let pdpt = PDPTEntry(pdpt);

        // PDPT (1 GB)
        //
        let pdpt_index = pdpt_index(physical_address);
        log::info!("PDPT index: {}", pdpt_index);

        // PDT (2 MB)
        //
        let pdt_index = pd_index(physical_address);
        log::info!("PDT index: {}", pdt_index);

        // PT (4 KB)
        //
        let pd_index = pt_index(physical_address);
        log::info!("PT index: {}", pd_index);

        // TODO: Implement table lookup

        todo!()
    }

    // TODO: Implement
    fn build_npt_entry(self: &AllocatedMemory<Self>, physical_address: Option<u64>) {
        // TODO: Allow to pass these through the parameter (pml4, pdpt, pdp, pd)
        let pml4 = PML4Entry(0);

        // let page_frame_number;
        // if let Some(physical_address) = physical_address {
        //     page_frame_number = pfn_from_pa!(physical_address);
        // } else {
        //     // TODO: Allocate npt entry and set in preallocated table in hook data
        //     //
        //     let sub_table = 0x0; // allocate_npt_entry
        //     page_frame_number = pfn_from_va!(sub_table);
        // }

        // Entry->Fields.Valid = TRUE;
        // Entry->Fields.Write = TRUE;
        // Entry->Fields.User = TRUE;
        // Entry->Fields.PageFrameNumber = pageFrameNumber;
    }

    fn map_2b(host_pa: u64, guest_va: u64) {}

    // TODO:
    // - Implement system()
    // - Split 2mb into 4kb pages
    // - Map 4kb guest physical address to 4kb host physical address (hooked) and vice versa
    // - Change permissions of other pages to RW and vice versa
    //    - Are there optimizations? Only borders/outside memory?

    // IMPORTANT: Can we cache it somehow?
    // TODO: Can we unroll the loop somehow? Probably quite big for 512 * 512 (262144 = 0x40000) -> Takes probably stil quite a long time
}
