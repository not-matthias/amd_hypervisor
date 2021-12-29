use crate::nt::addresses::physical_address;
use crate::nt::memory::AllocatedMemory;
use crate::svm::paging::PFN_MASK;
use elain::Align;
use x86::bits64::paging::{
    PAddr, PDEntry, PDFlags, PDPTEntry, PDPTFlags, PML4Entry, PML4Flags, PAGE_SIZE_ENTRIES, PD,
    PDPT,
};

#[repr(C, align(4096))]
pub struct NestedPageTable {
    pub pml4: [PML4Entry; 1],
    align_0: Align<4096>,

    pub pdp_entries: PDPT,
    align_1: Align<4096>,

    pub pd_entries: [PD; 512],
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
        (*self.ptr()).pml4[0] = PML4Entry::new(
            physical_address((*self.ptr()).pdp_entries.as_ptr() as _),
            PML4Flags::from_iter([PML4Flags::P, PML4Flags::RW, PML4Flags::US]),
        );

        // PDPT
        //
        for (i, pdp) in (*self.ptr()).pdp_entries.iter_mut().enumerate() {
            let pa = physical_address((*self.ptr()).pd_entries[i].as_ptr() as _);
            *pdp = PDPTEntry::new(
                pa,
                PDPTFlags::from_iter([PDPTFlags::P, PDPTFlags::RW, PDPTFlags::US]),
            );

            // PD
            //
            for (j, pd) in (*self.ptr()).pd_entries[i].iter_mut().enumerate() {
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

        self
    }
}
