use crate::nt::include::MmGetPhysicalAddress;
use crate::nt::memory::alloc_aligned;
use x86::bits64::paging::PDFlags;
use x86::bits64::paging::{PAddr, PDEntry, PML4Entry, PML4Flags};

#[repr(C, align(4096))]
pub struct NestedPageTableData {
    pub pml4_entries: [PML4Entry; 1],
    pub pdp_entries: [PML4Entry; 512],
    pub pde_entries: [[PDEntry; 512]; 512],
}

impl NestedPageTableData {
    pub fn new() -> Option<*mut Self> {
        // Allocate shared data
        //
        let memory = alloc_aligned(core::mem::size_of::<NestedPageTable>());
        if memory.is_none() {
            log::warn!("Failed to allocate nested page table data");
        }

        Some(memory? as *mut NestedPageTableData)
    }
}

pub struct NestedPageTable {
    pub data: *mut NestedPageTableData,
}

impl NestedPageTable {
    pub fn new() -> Option<Self> {
        Some(NestedPageTable {
            data: NestedPageTableData::new()?,
        })
    }

    unsafe fn get_physical_address(ptr: *mut u64) -> PAddr {
        let pdp_base_pa = *MmGetPhysicalAddress(ptr as _).QuadPart() as u64;

        PAddr::from(pdp_base_pa)
    }

    pub unsafe fn build(mut self) -> Self {
        // The US (User) bit of all nested page table entries to be translated
        // without #VMEXIT, as all guest accesses are treated as user accesses at
        // the nested level. Also, the RW (Write) bit of nested page table entries
        // that corresponds to guest page tables must be 1 since all guest page
        // table accesses are treated as write access. See "Nested versus Guest
        // Page Faults, Fault Ordering" for more details.
        //
        // Nested page tables built here set 1 to those bits for all entries, so
        // that all translation can complete without triggering #VMEXIT. This does
        // not lower security since security checks are done twice independently:
        // based on guest page tables, and nested page tables. See "Nested versus
        // Guest Page Faults, Fault Ordering" for more details.
        //
        let flags = PML4Flags::from_iter([PML4Flags::P, PML4Flags::RW, PML4Flags::US]);

        // Build only one PML4 entry. This entry has sub-tables that control up to
        // 512GB physical memory. PFN points to a base physical address of the page
        // directory pointer table.
        //
        let pdp_base_pa = Self::get_physical_address((*self.data).pdp_entries.as_mut_ptr() as _);

        // (*self.data).pml4_entries[0].set_page_frame_number(pdp_base_pa >> PAGE_SHIFT);
        (*self.data).pml4_entries[0] = PML4Entry::new(pdp_base_pa, flags);

        // TODO: Check if this is the same thing.

        // One PML4 entry controls 512 page directory pointer entries.
        //
        for i in 0..512 {
            // PFN points to a base physical address of the page directory table.
            //
            let flags = PML4Flags::from_iter([PML4Flags::P, PML4Flags::RW, PML4Flags::US]);

            let pde_address = &mut (*self.data).pde_entries[i][0] as *mut _ as _;
            let pde_base_pa = Self::get_physical_address(pde_address);

            (*self.data).pdp_entries[i] = PML4Entry::new(pde_base_pa, flags);

            // One page directory entry controls 512 page directory entries.
            //
            // We do not explicitly configure PAT in the NPT entry. The consequences
            // of this are:
            //
            // 1) pages whose PAT (Page Attribute Table) type is the
            // Write-Combining (WC) memory type could be treated as the
            // Write-Combining Plus (WC+) while it should be WC when the MTRR type
            // is either Write Protect (WP), Write-through (WT) or Write-back
            // (WB), and
            //
            // 2) pages whose PAT type is Uncacheable Minus (UC-)
            // could be treated as Cache Disabled (CD) while it should be
            // WC, when MTRR type is WC.
            //
            // While those are not desirable, this is acceptable given that 1) only
            // introduces additional cache snooping and associated performance
            // penalty, which would not be significant since WC+ still lets
            // processors combine multiple writes into one and avoid large
            // performance penalty due to frequent writes to memory without caching.
            // 2) might be worse but I have not seen MTRR ranges configured as WC
            // on testing, hence the unintentional UC- will just results in the same
            // effective memory type as what would be with UC.
            //
            // See "Memory Types" (7.4), for details of memory types,
            // "PAT-Register PA-Field Indexing", "Combining Guest and Host PAT
            // Types", and "Combining PAT and MTRR Types" for how the
            // effective memory type is determined based on Guest PAT type,
            // Host PAT type, and the MTRR type.
            //
            // The correct approach may be to look up the guest PTE and copy the
            // caching related bits (PAT, PCD, and PWT) when constructing NTP
            // entries for non RAM regions, so the combined PAT will always be the
            // same as the guest PAT type. This may be done when any issue manifests
            // with the current implementation.
            //
            for j in 0..512 {
                // PFN points to a base physical address of system physical address
                // to be translated from a guest physical address. Set the PS
                // (LargePage) bit to indicate that this is a large page and no
                // sub-table exists.
                //
                let translation_pa = (i * 512) + j;

                (*self.data).pde_entries[i][j] = PDEntry::new(
                    PAddr::from(translation_pa),
                    PDFlags::from_iter([PDFlags::P, PDFlags::RW, PDFlags::US, PDFlags::PS]),
                );
            }
        }

        self
    }
}
