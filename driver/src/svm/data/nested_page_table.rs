extern crate alloc;

use crate::nt::addresses::physical_address;
use crate::nt::memory::AllocatedMemory;
use crate::svm::paging::{AccessType, PFN_MASK, _1GB, _2MB, _512GB};
use crate::PhysicalMemoryDescriptor;
use elain::Align;
use x86::bits64::paging::{
    pd_index, pdpt_index, pml4_index, pt_index, PAddr, PDEntry, PDFlags, PDPTEntry, PDPTFlags,
    PML4Entry, PML4Flags, PTEntry, PTFlags, VAddr, BASE_PAGE_SIZE, LARGE_PAGE_SIZE,
    PAGE_SIZE_ENTRIES, PD, PDPT, PML4, PT,
};
use x86::msr::{rdmsr, IA32_APIC_BASE};

/// TODO: Detection Vector: Lookup page tables in physical memory
#[repr(C, align(4096))]
pub struct NestedPageTable {
    pub pml4: PML4,
    align_0: Align<4096>,

    pub pdp_entries: PDPT,
    align_1: Align<4096>,

    pub pd_entries: [PD; 512],
    align_2: Align<4096>,

    pub pt_entries: [[PT; 512]; 512],
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
                // This will calculate all the 2MB pages.
                //
                // Note, these values only appear, once you shl 21 and apply the PFN mask.
                // The list starts like this for i = 0 and j = 0-4:
                //
                // 0x0
                // 0x200000
                // 0x400000
                // 0x600000
                // 0x800000
                //
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

    pub fn identity_2mb() -> Option<AllocatedMemory<Self>> {
        log::info!("Building nested page tables with 2MB pages");

        let mut npt =
            AllocatedMemory::<Self>::alloc_aligned(core::mem::size_of::<NestedPageTable>())?;

        log::info!("Mapping 512GB of physical memory");
        for pa in (0.._512GB).step_by(_2MB) {
            npt.map_2mb(pa, pa, AccessType::ReadWriteExecute);
        }

        Some(npt)
    }

    pub fn identity_4kb() -> Option<AllocatedMemory<Self>> {
        log::info!("Building nested page tables with 4KB pages");

        let mut npt =
            AllocatedMemory::<Self>::alloc_aligned(core::mem::size_of::<NestedPageTable>())?;

        log::info!("Mapping 512GB of physical memory");
        for pa in (0.._512GB).step_by(BASE_PAGE_SIZE) {
            npt.map_4kb(pa, pa, AccessType::ReadWriteExecute);
        }

        Some(npt)
    }

    /// Splits a large 2MB page into 512 smaller 4KB pages.
    ///
    /// This is needed to apply more granular hooks and to reduce the number of page faults
    /// that occur when the guest tries to access a page that is hooked.
    ///
    /// See:
    /// - https://github.com/wbenny/hvpp/blob/master/src/hvpp/hvpp/ept.cpp#L245
    pub fn split_2mb_to_4kb(&mut self, guest_pa: u64) -> Option<()> {
        log::trace!("Splitting 2mb page into 4kb pages: {:x}", guest_pa);

        let guest_pa = VAddr::from(guest_pa);

        let pdpt_index = pdpt_index(guest_pa);
        let pd_index = pd_index(guest_pa);
        let pd_entry = &mut self.pd_entries[pdpt_index][pd_index];

        // We can only split large pages and not page directories.
        // If it's a page directory, it is already split.
        //
        if !pd_entry.is_page() {
            log::warn!("Tried to split a page directory: {:x}.", guest_pa);
            return Some(());
        }

        // Unmap the large page
        //
        Self::unmap_2mb(pd_entry);

        // Map the unmapped physical memory again to 4KB pages.
        //
        for i in 0..PAGE_SIZE_ENTRIES {
            let address = guest_pa.as_usize() + i * BASE_PAGE_SIZE;

            log::trace!("Mapping 4kb page: {:x}", address);

            self.map_4kb(address as _, address as _, AccessType::ReadWriteExecute);
        }

        Some(())
    }

    pub fn join_4kb_to_2mb(&mut self, guest_pa: u64) -> Option<()> {
        log::trace!("Joining 4kb pages into 2mb page: {:x}", guest_pa);

        let guest_pa = VAddr::from(guest_pa);

        let pdpt_index = pdpt_index(guest_pa);
        let pd_index = pd_index(guest_pa);
        let pd_entry = &mut self.pd_entries[pdpt_index][pd_index];

        if pd_entry.is_page() {
            log::warn!(
                "Tried to join a large page: {:x}. Only page directories can be joined.",
                guest_pa
            );
            return Some(());
        }

        // Unmap the page directory
        //
        Self::unmap_4kb(pd_entry);

        // Map the unmapped physical memory again to a 2MB large page.
        //
        log::trace!("Mapping 2mb page: {:x}", guest_pa);
        self.map_2mb(
            guest_pa.as_u64(),
            guest_pa.as_u64(),
            AccessType::ReadWriteExecute,
        );

        Some(())
    }

    pub fn map_2mb(&mut self, guest_pa: u64, host_pa: u64, access_type: AccessType) {
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
            // We already have the page frame number of the physical address, so we don't need
            // to calculate it on our own. Just pass it to the page directory entry.
            //
            *pd_entry = PDEntry::new(
                PAddr::from(host_pa.as_u64()),
                PDFlags::from_iter([PDFlags::P, PDFlags::RW, PDFlags::US, PDFlags::PS]),
            );
        }
    }

    // TODO: Make it more granular and merge duplicated code
    pub fn map_4kb(&mut self, guest_pa: u64, host_pa: u64, access_type: AccessType) {
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
            let pa = physical_address(self.pt_entries[pdpt_index][pd_index].as_ptr() as _);
            *pd_entry = PDEntry::new(
                PAddr::from(pa),
                PDFlags::from_iter([PDFlags::P, PDFlags::RW, PDFlags::US]),
            );
        }

        // PT (4 KB)
        //
        let pt_index = pt_index(guest_pa);
        let pt_entry = &mut self.pt_entries[pdpt_index][pd_index][pt_index];

        if !pt_entry.is_present() {
            // We already have the page frame number of the physical address, so we don't need
            // to calculate it on our own. Just pass it to the page table entry.
            //
            *pt_entry = PTEntry::new(
                PAddr::from(host_pa.as_u64()),
                PTFlags::from_iter([PTFlags::P, PTFlags::RW, PTFlags::US]),
            );
        }
    }

    fn unmap_2mb(entry: &mut PDEntry) {
        if !entry.is_present() {
            return;
        }

        // TODO: Do we need to iterate over the subtables?

        // Clear the flags
        //
        *entry = PDEntry(entry.address().as_u64());
    }

    fn unmap_4kb(entry: &mut PDEntry) {
        // TODO: We should probably either make this generic or recode the logic to also clear 4kb entries
        Self::unmap_2mb(entry);
    }

    pub fn change_all_permissions(&mut self, permission: AccessType) {
        // TODO: Only iterate up to max_pdpt_index

        // Set the permission for all the PDP entries.
        //
        for pdp_entry in self.pdp_entries.iter_mut() {
            let mut flags = pdp_entry.flags();
            match permission {
                AccessType::ReadWrite => {
                    flags.insert(PDPTFlags::RW);
                    flags.insert(PDPTFlags::XD);
                }
                AccessType::ReadWriteExecute => {
                    flags.insert(PDPTFlags::RW);
                    flags.remove(PDPTFlags::XD);
                }
            }

            *pdp_entry = PDPTEntry::new(pdp_entry.address(), flags);
        }
    }

    /// Changes the permission of a single page.
    ///
    pub fn change_page_permission(&mut self, guest_pa: u64, permission: AccessType) {
        log::info!(
            "Changing permission of guest page {:#x} to {:?}",
            guest_pa,
            permission
        );

        let guest_pa = VAddr::from(guest_pa);

        let pdpt_index = pdpt_index(guest_pa);
        let pd_index = pd_index(guest_pa);
        let pt_index = pt_index(guest_pa);

        let pd_entry = &mut self.pd_entries[pdpt_index][pd_index];
        if pd_entry.is_page() {
            log::info!("Changing the permission of a 2mb page is currently not supported.");
            return;
        }

        let pt_entry = &mut self.pt_entries[pdpt_index][pd_index][pt_index];
        assert!(pt_entry.is_present());

        let mut flags = pt_entry.flags();
        log::info!("Current flags: {:?}", flags);
        match permission {
            AccessType::ReadWrite => {
                flags.insert(PTFlags::RW);
                flags.insert(PTFlags::XD);
            }
            AccessType::ReadWriteExecute => {
                flags.insert(PTFlags::RW);
                flags.remove(PTFlags::XD);
            }
        };
        log::info!("New flags: {:?}", flags);

        *pt_entry = PTEntry::new(pt_entry.address(), flags);
    }

    /// Builds the nested page table to cover for the entire physical memory address space.
    ///
    #[deprecated(note = "This doesn't work at the current time. Use `identity` instead.")]
    pub fn system() -> Option<AllocatedMemory<Self>> {
        let desc = PhysicalMemoryDescriptor::new()?;
        let mut npt =
            AllocatedMemory::<Self>::alloc_aligned(core::mem::size_of::<NestedPageTable>())?;

        // NPT entries based on the physical memory ranges.
        //
        for range in desc.ranges {
            let base_address = range.base_page() * LARGE_PAGE_SIZE as u64;

            log::info!(
                "Mapping base_address: {:x}, page_count: {}",
                base_address,
                range.page_count()
            );
            for page_index in 0..range.page_count() {
                let physical_address = base_address + page_index * LARGE_PAGE_SIZE as u64;

                // log::info!("Mapping 2mb physical_address: {:x}", physical_address);

                npt.map_2mb(
                    physical_address,
                    physical_address,
                    AccessType::ReadWriteExecute,
                );
            }
        }

        // TODO: Do we need APIC base?
        // Map
        //
        let apic_base = unsafe { rdmsr(IA32_APIC_BASE) };
        // Bits 12:35
        let apic_base = apic_base & 0xFFFFF000; // TODO: Trust copilot or do it myself?
        let apic_base = apic_base * LARGE_PAGE_SIZE as u64;

        npt.map_2mb(apic_base, apic_base, AccessType::ReadWriteExecute);

        // Compute max PDPT index based on last descriptor entry that describes the highest pa.
        //
        let last_range = desc.ranges.last()?;

        let base_address = last_range.base_page() * LARGE_PAGE_SIZE as u64;
        let end = base_address + last_range.page_count() * LARGE_PAGE_SIZE as u64;

        const _1GB: u64 = 1024 * 1024 * 1024;
        macro round_to_size($length: expr, $alignment: expr) {
            ($length + $alignment - 1) & !($alignment - 1)
        }

        let max_pdp_index = round_to_size!(end, _1GB) / _1GB;
        log::info!("Max PDPT index: {}", max_pdp_index);
        log::info!("End: {:x}", end);

        Some(npt)
    }

    // TODO: Not yet working. Fix it.
    pub fn last_pdp_index(&self) -> Option<u64> {
        let desc = PhysicalMemoryDescriptor::new()?;

        let last_range = desc.ranges.last()?;
        let base_address = last_range.base_page() * LARGE_PAGE_SIZE as u64;
        let end = base_address + last_range.page_count() * LARGE_PAGE_SIZE as u64;

        macro round_to_size($length: expr, $alignment: expr) {
            ($length + $alignment - 1) & !($alignment - 1)
        }

        let max_pdp_index = round_to_size!(end, _1GB) / _1GB;
        log::info!("Max PDPT index: {}", max_pdp_index);
        log::info!("End: {:x}", end);

        Some(max_pdp_index)
    }

    // TODO:
    // - Map 4kb guest physical address to 4kb host physical address (hooked) and vice versa
    // - Change permissions of other pages to RW and vice versa
    //    - Are there optimizations? Only borders/outside memory?

    // IMPORTANT: Can we cache it somehow?
}
