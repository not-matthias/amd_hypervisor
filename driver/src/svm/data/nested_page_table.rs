use crate::nt::addresses::physical_address;
use elain::Align;
use x86::bits64::paging::{
    PAddr, PDEntry, PDFlags, PDPTEntry, PDPTFlags, PML4Entry, PML4Flags, MAXPHYADDR,
};

use crate::nt::memory::AllocatedMemory;
use crate::svm::paging::{page_to_pfn, LegacyPDE, LegacyPDPTE, LegacyPML4E};

#[repr(C, align(4096))]
pub struct NestedPageTable {
    pub pml4: [LegacyPML4E; 1],
    align_0: Align<4096>,

    pub pdp_entries: [LegacyPDPTE; 512],
    align_1: Align<4096>,

    pub pd_entries: [[LegacyPDE; 512]; 512],
    align_2: Align<4096>,
}

impl NestedPageTable {
    pub fn new() -> Option<AllocatedMemory<Self>> {
        AllocatedMemory::alloc_aligned(core::mem::size_of::<NestedPageTable>())
    }

    pub unsafe fn build(
        mut self: AllocatedMemory<NestedPageTable>,
    ) -> AllocatedMemory<NestedPageTable> {
        log::info!("Building nested page tables");

        // PML4
        //

        let pa = physical_address((*self.ptr()).pdp_entries.as_ptr() as _);

        let flags = PML4Flags::from_iter([PML4Flags::P, PML4Flags::RW, PML4Flags::US]);
        let pml4 = PML4Entry::new(pa, flags);

        let mut legacy = LegacyPML4E(0);
        legacy.set_page_frame_number(page_to_pfn!(pa));
        legacy.set_valid(1);
        legacy.set_write(1);
        legacy.set_user(1);

        if legacy.0 != pml4.0 {
            log::error!("PML4Entry: {:?} != {:?}", legacy.0, pml4.0);
        }
        // assert_eq!(legacy.0, pml4.0);

        (*self.ptr()).pml4[0] = legacy.into();

        // PDPTE
        //
        for (i, table_pdp) in (*self.ptr()).pdp_entries.iter_mut().enumerate() {
            let pa = physical_address((*self.ptr()).pd_entries[i].as_ptr() as _);

            let flags = PDPTFlags::from_iter([PDPTFlags::P, PDPTFlags::RW, PDPTFlags::US]);
            let pdp = PDPTEntry::new(pa, flags);

            let mut legacy_pdp = LegacyPML4E(0);
            legacy_pdp.set_page_frame_number(page_to_pfn!(pa));
            legacy_pdp.set_valid(1);
            legacy_pdp.set_write(1);
            legacy_pdp.set_user(1);

            if legacy_pdp.0 != pdp.0 {
                log::error!("PDPTEntry: {:?} != {:?}", legacy_pdp.0, pdp.0);
            }
            // assert_eq!(legacy_pdp.0, pdp.0);

            *table_pdp = legacy_pdp;

            // PDE
            //
            for (j, table_pd) in (*self.ptr()).pd_entries[i].iter_mut().enumerate() {
                let pa = i * (*self.ptr()).pd_entries[i].len() + j;
                let pa = pa as u64;

                // Mask to find the page frame number. We have to use this so that we can use `PDEntry`.
                // This is the same as using a `bitflags` struct and setting the bits `21-51`.
                //
                const ADDRESS_MASK: u64 = ((1 << MAXPHYADDR) - 1) & !0xfff;
                let pfn = pa << 21 & ADDRESS_MASK;

                let flags = PDFlags::from_iter([PDFlags::P, PDFlags::RW, PDFlags::US, PDFlags::PS]);
                let pd = PDEntry::new(PAddr::from(pfn), flags);

                let mut legacy_pd = LegacyPDE(0);
                legacy_pd.set_page_frame_number(pa);
                legacy_pd.set_valid(1);
                legacy_pd.set_write(1);
                legacy_pd.set_user(1);
                legacy_pd.set_large_page(1);

                if legacy_pd.0 != pd.0 {
                    log::error!("PDEntry({:x?}): {:?} != {:?}", pa, legacy_pd.0, pd.0);
                }
                // assert_eq!(legacy_pd.0, pd.0);

                *table_pd = legacy_pd;
            }
        }

        self
    }
}
