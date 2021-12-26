use x86::bits64::paging::{PAddr, PDFlags, PDPTEntry, PDPTFlags, PML4Flags};
use x86::bits64::paging::{PDEntry, PML4Entry};

use bitfield::bitfield;

bitfield! {
    /// See Figure 5-25. (2-Mbyte PDE—Long Mode)
    pub struct LegacyPDE(u64);
    pub get_valid, set_valid: 0, 0;                             // [0]
    pub get_write, set_write: 1, 1;                             // [1]
    pub get_user, set_user: 2, 2;                               // [2]
    pub get_write_through, set_write_through: 3, 3;             // [3]
    pub get_cache_disable, set_cache_disable: 4, 4;             // [4]
    pub get_accessed, set_accessed: 5, 5;                       // [5]
    pub get_dirty, set_dirty: 6, 6;                             // [6]
    pub get_large_page, set_large_page: 7, 7;                   // [7]
    pub get_global, set_global: 8, 8;                           // [8]
    pub get_avl, set_avl: 11, 9;                                // [9-11]
    pub get_pat, set_pat: 12, 12;                               // [12]
    // reserved                                                 // [13-20]
    pub get_page_frame_number, set_page_frame_number: 51, 21;   // [21-51]
    // reserved                                                 // [52-62]
    pub get_no_execute, set_no_execute: 63, 63;                 // [63]
}

bitfield! {
    /// See Figure 5-23. (2-Mbyte PML4E—Long Mode)
    pub struct LegacyPML4E(u64);
    pub get_valid, set_valid: 0, 0;                             // [0]
    pub get_write, set_write: 1, 1;                             // [1]
    pub get_user, set_user: 2, 2;                               // [2]
    pub get_write_through, set_write_through: 3, 3;             // [3]
    pub get_cache_disable, set_cache_disable: 4, 4;             // [4]
    pub get_accessed, set_accessed: 5, 5;                       // [5]
    // reserved                                                 // [6-8]
    pub get_avl, set_avl: 11, 9;                                // [9-11]
    pub get_page_frame_number, set_page_frame_number: 51, 12;   // [12-51]
    // reserved                                                 // [52-62]
    pub get_no_execute, set_no_execute: 63, 63;                 // [63]
}

#[repr(C, align(4096))]
pub struct NestedPageTableData {
    pub pml4_entries: [PML4Entry; 1],
    pub pdp_entries: [PML4Entry; 512],
    pub pde_entries: [[PDEntry; 512]; 512],
}

/// MAXPHYADDR, which is at most 52; (use CPUID for finding system value).
pub const MAXPHYADDR: u64 = 52;

/// Mask to find the physical address of an entry in a page-table.
const ADDRESS_MASK: u64 = ((1 << MAXPHYADDR) - 1) & !0xfff;

fn legacy_and_new_testing() {
    const PHYS_ADDR: u64 = 0x152338000;

    println!(
        "0x14f8b9008_u64 & ADDRESS_MASK: {:x?}",
        PHYS_ADDR & ADDRESS_MASK
    );
    println!(
        "0x14f8b9008_u64 >> PAGE_SHIFT: {:x?}",
        PHYS_ADDR >> PAGE_SHIFT << PAGE_SHIFT
    );

    let unaligned = PAddr::from(PHYS_ADDR);
    println!("unaligned: {:x?}", unaligned);

    let aligned_pa = unaligned.align_down_to_base_page();
    println!("aligned to base page: {:x?}", aligned_pa);

    // NEW
    //
    let entry = PML4Entry::new(
        aligned_pa,
        PML4Flags::from_iter([PML4Flags::P, PML4Flags::RW, PML4Flags::US]),
    );

    println!("NEW: {:x?}", entry);
    println!("NEW: {:x?}", entry.0);

    // LEGACY
    //
    pub const PAGE_SHIFT: u64 = 12;
    let mut legacy = LegacyPML4E(0);
    legacy.set_page_frame_number(PHYS_ADDR >> PAGE_SHIFT);

    legacy.set_valid(1);
    legacy.set_write(1);
    legacy.set_user(1);

    println!("LEGACY: {:x?}", legacy.0);
}

fn pml4() {
    const PHYS_ADDR: u64 = 0x0;

    let pa = PAddr::from(PHYS_ADDR);
    let entry = PML4Entry::new(
        pa,
        PML4Flags::from_iter([PML4Flags::P, PML4Flags::RW, PML4Flags::US]),
    );
    println!("pml4 entry: {:x?}", entry);
}

fn pde() {
    const PDE_PA: u64 = 0x143d86000;
    let pa = PAddr::from(PDE_PA);
    let entry = PML4Entry::new(
        pa,
        PML4Flags::from_iter([PML4Flags::P, PML4Flags::RW, PML4Flags::US]),
    );
    println!("pde entry: {:x?}", entry);
}

fn pde_entry(i: u64, j: u64) {
    let translation_pa: u64 = (i * 512) + j;
    let pt = PAddr::from(translation_pa);

    println!("[debug] base_page_offset: {:x}", pt.base_page_offset());

    println!("[debug] pt: {:x}", pt);
    println!("[debug] pt aligned: {:x}", pt.align_down_to_base_page());
    println!("[debug] pt_val: {:x}", pt & ADDRESS_MASK);
    println!("[debug] pt_val == pt.into(), {:x} == {:x}", pt & ADDRESS_MASK, pt.as_u64());
    println!("[debug] pt % BASE_SIZE == 0 => {:x?}", pt.as_u64() % 4096);

    // LEGACY
    //

    let mut entry = LegacyPDE(0);
    entry.set_page_frame_number(translation_pa);
    entry.set_valid(1);
    entry.set_write(1);
    entry.set_user(1);
    entry.set_large_page(1);

    println!("[LEGACY]: pd: {:x?}", entry.0);
    println!("[LEGACY]: pd: {:x?}", entry.get_page_frame_number());

    // NEW
    //

    let entry = PDEntry::new(
        pt.align_down_to_base_page(),
        PDFlags::from_iter([PDFlags::P, PDFlags::RW, PDFlags::US, PDFlags::PS]),
    );
    println!("[NEW]: pd: {:x?}", entry);
    println!("[NEW]: pd: {:x?}", entry.0);
    println!("[NEW]: pd: {:x?}", entry.address());
}

fn main() {
    println!("size: {:x?}", std::mem::size_of::<NestedPageTableData>());

    // // Page table testing
    // //
    // legacy_and_new_testing();
    //
    // // PML4E
    // //
    // pml4();
    //
    // // PDE
    // //
    // pde();

    // PD
    //
    pde_entry(123, 123);
}
