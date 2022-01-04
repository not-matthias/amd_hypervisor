use crate::nt::include::{MmGetPhysicalAddress, MmGetVirtualForPhysical};
use core::ops::{Deref, DerefMut};

use winapi::shared::ntdef::PHYSICAL_ADDRESS;
use x86::bits64::paging::{PAddr, BASE_PAGE_SHIFT};

pub struct PhysicalAddress(PAddr);

impl PhysicalAddress {
    pub fn from_pa(pa: u64) -> Self {
        Self(PAddr::from(pa))
    }

    pub fn from_pfn(pfn: u64) -> Self {
        Self(PAddr::from(pfn << BASE_PAGE_SHIFT))
    }

    pub fn from_va(va: u64) -> Self {
        Self(PAddr::from(Self::pa_from_va(va)))
    }

    /// Returns the virtual address of the current physical address.
    pub fn va(&self) -> *mut u64 {
        Self::va_from_pa(self.0.as_u64()) as *mut u64
    }

    pub fn pfn(&self) -> u64 {
        self.0.as_u64() >> BASE_PAGE_SHIFT
    }

    pub fn pa(&self) -> u64 {
        self.0.as_u64()
    }

    fn pa_from_va(va: u64) -> u64 {
        unsafe { *MmGetPhysicalAddress(va as _).QuadPart() as u64 }
    }

    fn va_from_pa(pa: u64) -> u64 {
        let mut physical_address: PHYSICAL_ADDRESS = unsafe { core::mem::zeroed() };
        unsafe { *(physical_address.QuadPart_mut()) = pa as i64 };

        unsafe { MmGetVirtualForPhysical(physical_address) as u64 }
    }
}

impl Deref for PhysicalAddress {
    type Target = PAddr;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for PhysicalAddress {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

// TODO: Replace this with `PhysicalAddress`.

pub fn physical_address(ptr: *const u64) -> PAddr {
    let physical_address = unsafe { *MmGetPhysicalAddress(ptr as _).QuadPart() } as u64;

    log::trace!("physical_address({:p}) = {:x}", ptr, physical_address);

    PAddr::from(physical_address)
}

pub fn aligned_physical_address(ptr: *mut u64) -> PAddr {
    physical_address(ptr).align_down_to_base_page()
}
