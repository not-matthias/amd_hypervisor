use crate::nt::include::MmGetPhysicalAddress;
use x86::bits64::paging::PAddr;

pub fn physical_address(ptr: *const u64) -> PAddr {
    let physical_address = unsafe { *MmGetPhysicalAddress(ptr as _).QuadPart() } as u64;

    log::trace!("physical address: {:x}", physical_address);

    PAddr::from(physical_address)
}

pub fn aligned_physical_address(ptr: *mut u64) -> PAddr {
    let physical_address = physical_address(ptr);

    PAddr::from(physical_address).align_down_to_base_page()
}
