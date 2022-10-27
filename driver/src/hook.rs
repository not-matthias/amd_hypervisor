use core::{
    mem, ptr,
    sync::atomic::{AtomicPtr, Ordering},
};

extern "C" {
    #[link_name = "llvm.addressofreturnaddress"]
    fn return_address() -> *const u64;
}

pub static ORIGINAL: AtomicPtr<u64> = AtomicPtr::new(ptr::null_mut());
pub fn mm_is_address_valid(ptr: u64) -> bool {
    log::info!("MmIsAddressValid called from {:#x}", unsafe {
        return_address().read_volatile()
    });

    // Call original
    //
    let fn_ptr = ORIGINAL.load(Ordering::Relaxed);
    let fn_ptr = unsafe { mem::transmute::<_, fn(u64) -> bool>(fn_ptr) };

    fn_ptr(ptr)
}
