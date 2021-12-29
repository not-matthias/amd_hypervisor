use x86::bits64::paging::MAXPHYADDR;

pub const PAGE_SHIFT: u64 = 12;
pub const PAGE_SIZE: usize = 0x1000;
pub const PAGE_MASK: usize = !(PAGE_SIZE - 1);
pub const PFN_MASK: u64 = ((1 << MAXPHYADDR) - 1) & !0xfff;

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
