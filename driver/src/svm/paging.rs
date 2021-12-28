use bitfield::bitfield;

pub const PAGE_SHIFT: u64 = 12;
pub const PAGE_SIZE: usize = 0x1000;
pub const PAGE_MASK: usize = !(PAGE_SIZE - 1);

/// Converts a page address to a page frame number.
pub macro page_to_pfn($page: expr) {
    ($page >> crate::svm::paging::PAGE_SHIFT) as u64
}

/// Aligns the specified virtual address to a page.
///
/// # Example
/// ```
/// let page = page_align!(4097);
/// assert_eq!(page, 4096);
/// ```
///
/// # Credits
/// // See: https://stackoverflow.com/questions/20771394/how-to-understand-the-macro-of-page-align-in-kernel/20771666
pub macro page_align($virtual_address:expr) {
    ($virtual_address + crate::svm::paging::PAGE_SIZE - 1) & crate::svm::paging::PAGE_MASK
}

pub type LegacyPDPTE = LegacyPML4E;

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
