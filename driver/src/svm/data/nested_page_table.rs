use crate::{
    svm::paging::{AccessType, PFN_MASK, _2MB, _512GB},
    utils::addresses::physical_address,
    PhysicalMemoryDescriptor,
};
use alloc::boxed::Box;
use elain::Align;
use x86::bits64::paging::{
    pd_index, pdpt_index, pml4_index, pt_index, PAddr, PDEntry, PDFlags, PDPTEntry, PDPTFlags,
    PML4Entry, PML4Flags, PTEntry, VAddr, BASE_PAGE_SIZE, PAGE_SIZE_ENTRIES, PD, PDPT, PML4, PT,
};

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
const_assert_eq!(core::mem::size_of::<NestedPageTable>(), 0x40202000);
const_assert!(core::mem::align_of::<NestedPageTable>() == 4096);

impl NestedPageTable {
    /// Creates the 2MB identity page table. Maps every guest physical address
    /// to the same host physical address.
    /// This means physical address 0x4000 in the guest will point to the
    /// physical memory 0x4000 in the host.
    ///
    ///
    /// # How it works
    ///
    /// We create a page table with **2MB** instead of **4KB** pages. There's
    /// multiple reasons for that:
    /// - Smaller page table.
    /// - Iterating is faster since we remove 1 iteration.
    ///
    /// Pros:
    /// - Easier to implement.
    /// - Faster to iterate (3 levels instead of 4).
    ///
    /// Cons:
    /// - We probably don't need access to 512 GB of physical memory.
    /// - Hooking 2MB pages is inconvenient, because we would get tons of ept
    ///   violations.
    ///
    /// # Other implementations
    ///
    /// Even though other hypervisors might be built for Intel processors, they
    /// still need to build some kind of [SLAT](https://en.wikipedia.org/wiki/Second_Level_Address_Translation) (Second Level Address Translation Table).
    ///
    /// Here's a list of useful references in popular projects:
    /// - [hvpp](https://github.com/wbenny/hvpp/blob/master/src/hvpp/hvpp/ept.cpp#L41)
    /// - [gbhv](https://github.com/Gbps/gbhv/blob/master/gbhv/ept.c#L167)
    pub fn identity() -> Box<Self> {
        log::info!("Building nested page tables");

        let npt: NestedPageTable = unsafe { core::mem::zeroed() };
        let mut npt = Box::new(npt);

        // PML4
        //
        npt.pml4[0] = PML4Entry::new(
            physical_address(npt.pdp_entries.as_ptr() as _),
            PML4Flags::from_iter([PML4Flags::P, PML4Flags::RW, PML4Flags::US]),
        );

        // PDPT
        //
        // Note: We have to use unsafe here to make sure that we can get access to a
        // mutable reference to the pdp entry. Otherwise we couldn't iterate
        // over the pd entries, since there already exists a mutable reference.
        //
        // Why do we need this? Because the arrays are both stored inside `self`, there
        // could be accesses to other arrays that we are currently iterating
        // over. This is NOT the case here, so we can use unsafe.
        //
        for (i, pdp) in unsafe {
            (*(npt.as_mut() as *mut NestedPageTable))
                .pdp_entries
                .iter_mut()
                .enumerate()
        } {
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

                // Mask to find the page frame number. We have to use this so that we can use
                // `PDEntry`. This is the same as using a `bitflags` struct and
                // setting the bits `21-51`.
                //
                let pfn = pa << 21 & PFN_MASK;

                *pd = PDEntry::new(
                    PAddr::from(pfn),
                    PDFlags::from_iter([PDFlags::P, PDFlags::RW, PDFlags::US, PDFlags::PS]),
                );
            }
        }

        npt
    }

    pub fn identity_2mb(access_type: AccessType) -> Box<Self> {
        log::info!("Building nested page tables with 2MB pages");

        let npt = Box::<Self>::new_zeroed();
        let mut npt = unsafe { npt.assume_init() };

        log::info!("Mapping 512GB of physical memory");
        for pa in (0.._512GB).step_by(_2MB) {
            npt.map_2mb(pa, pa, access_type);
        }

        npt
    }

    pub fn identity_4kb(access_type: AccessType) -> Box<NestedPageTable> {
        log::info!("Building nested page tables with 4KB pages");

        // TODO: Fix
        let mut npt = unsafe { Box::<Self>::new_zeroed().assume_init() };

        log::info!("Mapping 512GB of physical memory");
        for pa in (0.._512GB).step_by(BASE_PAGE_SIZE) {
            npt.map_4kb(pa, pa, access_type);
        }

        npt
    }

    /// Builds the nested page table to cover for the entire physical memory
    /// address space.
    #[deprecated(note = "This doesn't work at the current time. Use `identity` instead.")]
    pub fn system(access_type: AccessType) -> Box<NestedPageTable> {
        let desc = PhysicalMemoryDescriptor::new();
        let mut npt = unsafe { Box::<Self>::new_zeroed().assume_init() };

        for pa in (0..desc.total_size()).step_by(_2MB) {
            npt.map_2mb(pa as u64, pa as u64, access_type);
        }

        // Map
        //
        // let apic_base = unsafe { rdmsr(IA32_APIC_BASE) };
        // // Bits 12:35
        // let apic_base = apic_base & 0xFFFFF000;
        // let apic_base = apic_base * LARGE_PAGE_SIZE as u64;
        //
        // npt.map_2mb(apic_base, apic_base, access_type);

        npt
    }

    //
    //

    /// Splits a large 2MB page into 512 smaller 4KB pages.
    ///
    /// This is needed to apply more granular hooks and to reduce the number of
    /// page faults that occur when the guest tries to access a page that is
    /// hooked.
    ///
    /// See:
    /// - https://github.com/wbenny/hvpp/blob/master/src/hvpp/hvpp/ept.cpp#L245
    pub fn split_2mb_to_4kb(&mut self, guest_pa: u64, access_type: AccessType) {
        log::trace!("Splitting 2mb page into 4kb pages: {:x}", guest_pa);

        let guest_pa = VAddr::from(guest_pa);

        let pdpt_index = pdpt_index(guest_pa);
        let pd_index = pd_index(guest_pa);
        let pd_entry = &mut self.pd_entries[pdpt_index][pd_index];

        // We can only split large pages and not page directories.
        // If it's a page directory, it is already split.
        //
        if !pd_entry.is_page() {
            log::trace!("Page is already split: {:x}.", guest_pa);
            return;
        }

        // Unmap the large page
        //
        Self::unmap_2mb(pd_entry);

        // Map the unmapped physical memory again to 4KB pages.
        //
        for i in 0..PAGE_SIZE_ENTRIES {
            let address = guest_pa.as_usize() + i * BASE_PAGE_SIZE;
            self.map_4kb(address as _, address as _, access_type);
        }
    }

    pub fn join_4kb_to_2mb(&mut self, guest_pa: u64, access_type: AccessType) -> Option<()> {
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
        self.map_2mb(guest_pa.as_u64(), guest_pa.as_u64(), access_type);

        Some(())
    }

    //
    //

    pub(crate) fn map_2mb(&mut self, guest_pa: u64, host_pa: u64, access_type: AccessType) {
        self.map_pml4(guest_pa, access_type);
        self.map_pdpt(guest_pa, access_type);
        self.map_pde(guest_pa, host_pa, access_type);
    }

    pub(crate) fn map_4kb(&mut self, guest_pa: u64, host_pa: u64, access_type: AccessType) {
        self.map_pml4(guest_pa, access_type);
        self.map_pdpt(guest_pa, access_type);
        self.map_pdt(guest_pa, access_type);
        self.map_pt(guest_pa, host_pa, access_type);
    }

    fn map_pml4(&mut self, guest_pa: u64, access_type: AccessType) {
        let pml4_index = pml4_index(VAddr::from(guest_pa));
        let pml4_entry = &mut self.pml4[pml4_index];

        if !pml4_entry.is_present() {
            *pml4_entry = PML4Entry::new(
                physical_address(self.pdp_entries.as_ptr() as _),
                access_type.pml4_flags(),
            );
        }
    }

    fn map_pdpt(&mut self, guest_pa: u64, access_type: AccessType) {
        let pdpt_index = pdpt_index(VAddr::from(guest_pa));
        let pdpt_entry = &mut self.pdp_entries[pdpt_index];

        if !pdpt_entry.is_present() {
            let pa = physical_address(self.pd_entries[pdpt_index].as_ptr() as _);
            *pdpt_entry = PDPTEntry::new(pa, access_type.pdpt_flags());
        }
    }

    fn map_pdt(&mut self, guest_pa: u64, access_type: AccessType) {
        let pdpt_index = pdpt_index(VAddr::from(guest_pa));
        let pd_index = pd_index(VAddr::from(guest_pa));
        let pd_entry = &mut self.pd_entries[pdpt_index][pd_index];

        if !pd_entry.is_present() {
            let pa = physical_address(self.pt_entries[pdpt_index][pd_index].as_ptr() as _);

            *pd_entry = PDEntry::new(pa, access_type.pd_flags());
        }
    }

    fn map_pde(&mut self, guest_pa: u64, host_pa: u64, access_type: AccessType) {
        let pdpt_index = pdpt_index(VAddr::from(guest_pa));
        let pd_index = pd_index(VAddr::from(guest_pa));
        let pd_entry = &mut self.pd_entries[pdpt_index][pd_index];

        if !pd_entry.is_present() {
            // We already have the page frame number of the physical address, so we don't
            // need to calculate it on our own. Just pass it to the page
            // directory entry.
            //
            let flags = access_type.pd_flags() | PDFlags::PS;
            *pd_entry = PDEntry::new(PAddr::from(host_pa), flags);
        }
    }

    fn map_pt(&mut self, guest_pa: u64, host_pa: u64, access_type: AccessType) {
        let pdpt_index = pdpt_index(VAddr::from(guest_pa));
        let pd_index = pd_index(VAddr::from(guest_pa));
        let pt_index = pt_index(VAddr::from(guest_pa));
        let pt_entry = &mut self.pt_entries[pdpt_index][pd_index][pt_index];

        if !pt_entry.is_present() {
            // We already have the page frame number of the physical address, so we don't
            // need to calculate it on our own. Just pass it to the page table
            // entry.
            //
            *pt_entry = PTEntry::new(PAddr::from(host_pa), access_type.pt_flags());
        }
    }

    //
    //

    fn unmap_2mb(entry: &mut PDEntry) {
        if !entry.is_present() {
            return;
        }

        // Clear the flags
        //
        *entry = PDEntry::new(entry.address(), PDFlags::empty());
    }

    fn unmap_4kb(entry: &mut PDEntry) {
        Self::unmap_2mb(entry);
    }

    //
    //

    /// Changes the permission of a single page (can be 2mb or 4kb).
    pub fn change_page_permission(&mut self, guest_pa: u64, host_pa: u64, access_type: AccessType) {
        log::trace!(
            "Changing permission of guest page {:#x} to {:?}",
            guest_pa,
            access_type
        );

        let guest_pa = VAddr::from(guest_pa);
        let host_pa = PAddr::from(host_pa);

        let pml4_index = pml4_index(guest_pa);
        let pdpt_index = pdpt_index(guest_pa);
        let pd_index = pd_index(guest_pa);
        let pt_index = pt_index(guest_pa);

        self.pml4[pml4_index] =
            PML4Entry::new(self.pml4[pml4_index].address(), access_type.pml4_flags());

        self.pdp_entries[pdpt_index] = PDPTEntry::new(
            self.pdp_entries[pdpt_index].address(),
            access_type.pdpt_flags(),
        );

        let pd_entry = &mut self.pd_entries[pdpt_index][pd_index];
        if pd_entry.is_page() {
            log::trace!("Changing the permissions of a 2mb page");

            *pd_entry = PDEntry::new(host_pa, access_type.modify_2mb(pd_entry.flags()));
        } else {
            log::trace!("Changing the permissions of a 4kb page");

            *pd_entry = PDEntry::new(pd_entry.address(), access_type.pd_flags());

            let pt_entry = &mut self.pt_entries[pdpt_index][pd_index][pt_index];
            let flags = access_type.modify_4kb(pt_entry.flags());
            let entry = PTEntry::new(host_pa, flags);

            *pt_entry = entry;
        }
    }

    /// Should only be used for debugging
    pub fn print_page_permission(&mut self, guest_pa: u64) {
        let guest_pa = VAddr::from(guest_pa);

        let pdpt_index = pdpt_index(guest_pa);
        let pd_index = pd_index(guest_pa);
        let pt_index = pt_index(guest_pa);

        let pd_entry = &self.pd_entries[pdpt_index][pd_index];
        let pt_entry = &self.pt_entries[pdpt_index][pd_index][pt_index];
        log::info!("PDEntry: {:x?}, PTEntry: {:x?}", pd_entry, pt_entry);
    }

    pub fn last_pdp_index(&self) -> usize {
        PhysicalMemoryDescriptor::new().total_size_in_gb() + 1
    }
}
