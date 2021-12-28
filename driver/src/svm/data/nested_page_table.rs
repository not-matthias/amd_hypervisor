use crate::nt::addresses::physical_address;
use elain::Align;

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
        let mut pml4 = LegacyPML4E(0);
        let pa = physical_address((*self.ptr()).pdp_entries.as_ptr() as _);
        pml4.set_page_frame_number(page_to_pfn!(pa));
        pml4.set_valid(1);
        pml4.set_write(1);
        pml4.set_user(1);

        (*self.ptr()).pml4[0] = pml4.into();

        // PDPTE
        //
        for (i, table_pdp) in (*self.ptr()).pdp_entries.iter_mut().enumerate() {
            let pa = physical_address((*self.ptr()).pd_entries[i].as_ptr() as _);
            let mut pdp = LegacyPML4E(0);
            pdp.set_page_frame_number(page_to_pfn!(pa));
            pdp.set_valid(1);
            pdp.set_write(1);
            pdp.set_user(1);

            *table_pdp = pdp.into();

            // PDE
            //
            for (j, table_pd) in (*self.ptr()).pd_entries[i].iter_mut().enumerate() {
                let pa = i * (*self.ptr()).pd_entries[i].len() + j;

                let mut pd = LegacyPDE(0);
                pd.set_page_frame_number(pa as _);
                pd.set_valid(1);
                pd.set_write(1);
                pd.set_user(1);
                pd.set_large_page(1);

                *table_pd = pd.into();
            }
        }

        self
    }
}
