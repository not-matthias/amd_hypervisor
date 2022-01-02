extern crate alloc;

use crate::nt::addresses::physical_address;
use crate::nt::memory::AllocatedMemory;
use crate::svm::paging::{AccessType, PFN_MASK};
use crate::PhysicalMemoryDescriptor;
use alloc::vec::Vec;
use elain::Align;
use x86::bits64::paging::{
    pd_index, pdpt_index, pml4_index, pt_index, PAddr, PDEntry, PDFlags, PDPTEntry, PDPTFlags,
    PML4Entry, PML4Flags, VAddr, BASE_PAGE_SIZE, PAGE_SIZE_ENTRIES, PD, PDPT, PML4,
};

pub struct DynamicNpt {
    pml4: Vec<PML4>,
    pdpt: Vec<PDPT>,
    pd: Vec<PD>,
}

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
        npt.pml4[0] = PML4Entry::new(
            physical_address(npt.pdp_entries.as_ptr() as _),
            PML4Flags::from_iter([PML4Flags::P, PML4Flags::RW, PML4Flags::US]),
        );

        // PDPT
        //
        // Note: We have to use unsafe here to make sure that we can get access to a mutable reference
        // to the pdp entry. Otherwise we couldn't iterate over the pd entries, since there already
        // exists a mutable reference.
        //
        // Why do we need this? Because the arrays are both stored inside `self`, there could be
        // accesses to other arrays that we are currently iterating over. This is NOT the case
        // here, so we can use unsafe.
        //
        for (i, pdp) in unsafe { (*npt.inner().as_ptr()).pdp_entries.iter_mut().enumerate() } {
            let pa = physical_address(npt.pd_entries[i].as_ptr() as _);
            *pdp = PDPTEntry::new(
                pa,
                PDPTFlags::from_iter([PDPTFlags::P, PDPTFlags::RW, PDPTFlags::US]),
            );

            // PD
            //
            for (j, pd) in npt.pd_entries[i].iter_mut().enumerate() {
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

    pub fn identity_new() -> Option<AllocatedMemory<Self>> {
        log::info!("Building nested page tables");

        let mut npt =
            AllocatedMemory::<Self>::alloc_aligned(core::mem::size_of::<NestedPageTable>())?;

        const _512GB: u64 = 512 * 1024 * 1024 * 1024;

        for pa in (0.._512GB).step_by(BASE_PAGE_SIZE) {
            npt.map_2mb(pa, pa, AccessType::READ_WRITE_EXCUTE);
        }

        Some(npt)
    }

    /// Builds the nested page table to cover for the entire physical memory address space.
    ///
    ///
    pub fn system() -> Option<AllocatedMemory<Self>> {
        let desc = PhysicalMemoryDescriptor::new()?;
        let mut npt =
            AllocatedMemory::<Self>::alloc_aligned(core::mem::size_of::<NestedPageTable>())?;

        // NPT entries based on the physical memory ranges.
        //
        for range in desc.ranges {
            let base_address = range.base_page() * BASE_PAGE_SIZE as u64;

            for page_index in 0..range.page_count() {
                let address = base_address + page_index * BASE_PAGE_SIZE as u64;

                npt.map_2mb(address, address, AccessType::READ_WRITE_EXCUTE);
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
        // let last_range = desc.ranges.last().unwrap();
        // let _base_address = last_range.base_page() * BASE_PAGE_SIZE as u64;
        // // let _max_pdp_index =
        // //     (base_address + last_range.page_count() * BASE_PAGE_SIZE as u64).pdp_index();
        // // TODO: ROUND_TO_SIZE
        // // maxPpeIndex = ROUND_TO_SIZE(baseAddr + currentRun->PageCount * PAGE_SIZE,
        // //                             oneGigabyte) / oneGigabyte;

        Some(npt)
    }

    #[inline(always)]
    fn map_2mb(&mut self, guest_pa: u64, host_pa: u64, access_type: AccessType) {
        // TODO: Use access_type
        let _ = access_type;

        let guest_pa = VAddr::from(guest_pa);
        let host_pa = VAddr::from(host_pa);

        // PML4 (512 GB)
        //
        let pml4_index = pml4_index(guest_pa);
        let pml4_entry = &mut self.pml4[pml4_index];

        if !pml4_entry.is_present() {
            *pml4_entry = PML4Entry::new(
                physical_address(self.pdp_entries.as_ptr() as _),
                PML4Flags::from_iter([PML4Flags::P, PML4Flags::RW, PML4Flags::US]),
            );
        }

        // PDPT (1 GB)
        //
        let pdpt_index = pdpt_index(guest_pa);
        let pdpt_entry = &mut self.pdp_entries[pdpt_index];

        if !pdpt_entry.is_present() {
            let pa = physical_address(self.pd_entries[pdpt_index].as_ptr() as _);
            *pdpt_entry = PDPTEntry::new(
                pa,
                PDPTFlags::from_iter([PDPTFlags::P, PDPTFlags::RW, PDPTFlags::US]),
            );
        }

        // PD (2 MB)
        //
        let pd_index = pd_index(guest_pa);
        let pd_entry = &mut self.pd_entries[pdpt_index][pd_index];

        if !pd_entry.is_present() {
            // In 2MB pages, the PDE contains the 20 bit offset to the physical address. Instead of
            // using `pt_index` which returns the last 12 bits, we need to calculate the offset ourselves.
            //
            // See `5.3.4 2-Mbyte Page Translation` for more information.
            //
            let mask = (1 << 20) - 1; // 0xfffff = 0b11111111111111111111
            let page_offset = host_pa.as_u64() & mask;

            // TODO: Does that even work? We use host_pa for the other addresses but here we use the
            //       physical address? What if guest_pa is in pdpt[3] and host_pa is in pdpt[4]?
            //
            // I guess they have to be just right next to each other so that we can choose the correct page.
            //

            // TODO: Why do we need to do this?
            let pfn = page_offset << 21 & PFN_MASK;

            *pd_entry = PDEntry::new(
                PAddr::from(pfn),
                PDFlags::from_iter([PDFlags::P, PDFlags::RW, PDFlags::US, PDFlags::PS]),
            );
        }
    }

    fn build_sub_tables(&mut self, physical_address: u64) -> () {
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

    fn change_permissions(&mut self, _permission: ()) {
        // TODO: Iterate over all or only pml4?
    }

    // TODO:
    // - Implement system()
    // - Split 2mb into 4kb pages
    // - Map 4kb guest physical address to 4kb host physical address (hooked) and vice versa
    // - Change permissions of other pages to RW and vice versa
    //    - Are there optimizations? Only borders/outside memory?

    // IMPORTANT: Can we cache it somehow?
    // TODO: Can we unroll the loop somehow? Probably quite big for 512 * 512 (262144 = 0x40000) -> Takes probably stil quite a long time
}
