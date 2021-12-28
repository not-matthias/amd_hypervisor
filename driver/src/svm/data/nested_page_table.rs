use crate::nt::addresses::aligned_physical_address;

use crate::nt::memory::AllocatedMemory;
use crate::svm::paging::LegacyPDE;
use x86::bits64::paging::PDPTEntry;
use x86::bits64::paging::{PDPTFlags, PML4Entry, PML4Flags};

#[repr(C, align(4096))]
pub struct NestedPageTable {
    pub pml4_entries: [PML4Entry; 1],
    pub pdp_entries: [PDPTEntry; 512],
    pub pd_entries: [[LegacyPDE; 512]; 512],
}

impl NestedPageTable {
    pub fn new() -> Option<AllocatedMemory<Self>> {
        AllocatedMemory::alloc_aligned(core::mem::size_of::<NestedPageTable>())
    }

    pub unsafe fn build(
        mut self: AllocatedMemory<NestedPageTable>,
    ) -> AllocatedMemory<NestedPageTable> {
        log::info!("Building nested page tables");

        let pdp_base_pa = aligned_physical_address((*self.ptr()).pdp_entries.as_mut_ptr() as _);

        let flags = PML4Flags::from_iter([PML4Flags::P, PML4Flags::RW, PML4Flags::US]);
        let entry = PML4Entry::new(pdp_base_pa, flags);
        (*self.ptr()).pml4_entries[0] = entry;

        // One PML4 entry controls 512 page directory pointer entries.
        //
        for i in 0..512 {
            // log::trace!("Setting pdp entry {}", i);

            let pde_address = &mut (*self.ptr()).pd_entries[i][0];
            let pde_address = pde_address as *mut LegacyPDE as *mut u64;
            let pde_base_pa = aligned_physical_address(pde_address);

            let flags = PDPTFlags::from_iter([PDPTFlags::P, PDPTFlags::RW, PDPTFlags::US]);
            let entry = PDPTEntry::new(pde_base_pa, flags);

            (*self.ptr()).pdp_entries[i] = entry;

            for j in 0..512 {
                let translation_pa = (i * 512) + j;

                // TODO: Figure out how to use x86_64::PDEntry for this (fails due to misalignment)
                // (*self.data).pde_entries[i][j] = PDEntry::new(
                //     PAddr::from(translation_pa),
                //     PDFlags::from_iter([PDFlags::P, PDFlags::RW, PDFlags::US, PDFlags::PS]),
                // );

                (*self.ptr()).pd_entries[i][j].set_page_frame_number(translation_pa as u64);
                (*self.ptr()).pd_entries[i][j].set_valid(1);
                (*self.ptr()).pd_entries[i][j].set_write(1);
                (*self.ptr()).pd_entries[i][j].set_user(1);
                (*self.ptr()).pd_entries[i][j].set_large_page(1);
            }
        }

        self
    }
}

impl Drop for NestedPageTable {
    fn drop(&mut self) {
        // Do nothing
    }
}
