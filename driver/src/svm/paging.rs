use crate::nt::addresses::PhysicalAddress;


use x86::bits64::paging::{PML4Entry, MAXPHYADDR};

pub const _512GB: u64 = 512 * 1024 * 1024 * 1024;
pub const _1GB: u64 = 1024 * 1024 * 1024;
pub const _2MB: usize = 2 * 1024 * 1024;
pub const _4KB: usize = 4 * 1024;

// TODO: Replaec with BASE_PAGE_SIZE
pub const PAGE_SHIFT: u64 = 12;
pub const PAGE_SIZE: usize = 0x1000;
pub const PAGE_MASK: usize = !(PAGE_SIZE - 1);
pub const PFN_MASK: u64 = ((1 << MAXPHYADDR) - 1) & !0xfff;

#[derive(Debug)]
pub enum AccessType {
    ReadWrite,
    ReadWriteExecute,
}

pub macro pa_from_pfn(pfn: u64) {
    use crate::svm::paging::PAGE_SHIFT;

    ($pfn << PAGE_SHIFT)
}

pub macro va_from_pa($pa: expr) {
    let mut pa: PHYSICAL_ADDRESS = unsafe { core::mem::zeroed() };
    unsafe { *(pa.QuadPart_mut()) = $pa };

    unsafe { MmGetVirtualForPhysical(pa) }
}

pub macro va_from_pfn($pfn: expr) {{
    use crate::nt::include::MmGetVirtualForPhysical;
    use crate::svm::paging::pa_from_pfn;
    use crate::svm::paging::va_from_pa;
    use winapi::shared::ntdef::PHYSICAL_ADDRESS;

    let physical_address = pa_from_pfn!($pfn) as _;
    va_from_pa!(physical_address)
}}

// TODO: Return pfn or physical address that has been shifted?
pub macro pfn_from_pa($pa: expr) {
    ($pa >> PAGE_SHIFT) & PFN_MASK
}

/// Converts a page address to a page frame number.
pub macro page_to_pfn($page: expr) {
    ($page >> crate::svm::paging::PAGE_SHIFT) as u64
}

/// Calculates how many pages are required to hold the specified number of bytes.
pub macro bytes_to_pages($bytes: expr) {
    // ((($bytes) >> crate::svm::paging::PAGE_SHIFT) + ((($bytes) & (crate::svm::paging::PAGE_SIZE - 1)) != 0))

    ($bytes >> crate::svm::paging::PAGE_SHIFT) as usize
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

pub trait PagingHelper {
    /// Returns the page frame number of the current item.
    fn pfn(&self) -> u64;

    /// Returns the physical address of the current item.
    fn pa_from_pfn(&self) -> PhysicalAddress {
        PhysicalAddress::from_pfn(self.pfn())
    }

    fn subtable(&self) -> *mut u64 {
        self.pa_from_pfn().va()
    }
}

impl PagingHelper for PML4Entry {
    fn pfn(&self) -> u64 {
        self.address() & PFN_MASK
    }
}
