use crate::utils::{function_hook::FunctionHook, nt::MmIsAddressValid, ptr::Pointer};
use winapi::{
    shared::ntdef::{NTSTATUS, PULONG, ULONG},
    um::winnt::PVOID,
};

pub static mut ZWQSI_ORIGINAL: Option<Pointer<*mut u64>> = None;
pub fn zw_query_system_information(
    system_information_class: u32, system_information: PVOID, system_information_length: ULONG,
    return_length: PULONG,
) -> NTSTATUS {
    log::info!(
        "Called zw_query_system_information({:?}, {:x}, {:x}, {:p})",
        system_information_class,
        system_information as u64,
        system_information_length,
        return_length
    );

    // Call original
    //
    let fn_ptr = unsafe {
        core::mem::transmute::<_, fn(u32, PVOID, ULONG, PULONG) -> NTSTATUS>(
            ZWQSI_ORIGINAL.as_ref().unwrap(),
        )
    };

    fn_ptr(
        system_information_class,
        system_information,
        system_information_length,
        return_length,
    )
}

pub static mut EAPWT_ORIGINAL: Option<Pointer<FunctionHook>> = None;
pub fn ex_allocate_pool_with_tag(pool_tag: u32, number_of_bytes: u64, tag: u32) -> *mut u64 {
    log::info!(
        "Called ex_allocate_pool({:x}, {:x}, {:x})",
        pool_tag,
        number_of_bytes,
        tag
    );

    // Call original
    //
    log::info!("Calling original.");
    let fn_ptr = unsafe {
        core::mem::transmute::<_, fn(u32, u64, u32) -> *mut u64>(
            EAPWT_ORIGINAL.as_ref().unwrap().trampoline_address(),
        )
    };
    fn_ptr(pool_tag, number_of_bytes, tag)
}

pub static mut MMIAV_ORIGINAL: Option<Pointer<FunctionHook>> = None;
pub fn mm_is_address_valid(ptr: u64) -> bool {
    log::info!("Called mm_is_address_valid({:x})", ptr);

    // Call original
    //
    let fn_ptr = unsafe {
        core::mem::transmute::<_, fn(u64) -> bool>(
            MMIAV_ORIGINAL.as_ref().unwrap().trampoline_address(),
        )
    };

    fn_ptr(ptr)
}

// This can't be in the same page as the hook handler.
#[link_section = ".custom$test_hooks"]
#[inline(never)]
pub fn test_hooks() {
    // Test zw_query_system_information
    //
    // log::info!(
    //     "kernel debugger present: {:?}",
    //     protection::misc::is_kernel_debugger_present()
    // );

    // Test ex_allocate_pool_with_tag
    //
    // log::info!("EAPWT_ORIGINAL: {:?}", unsafe {
    //     EAPWT_ORIGINAL.as_ref().unwrap().as_ptr()
    // });
    // let ptr = unsafe { ExAllocatePoolWithTag(NonPagedPool as _, 0x20, 0xABCD) };
    // unsafe { ExFreePool(ptr as _) };

    // Test MmIsAddressValid
    //
    log::info!("Is address valid: {:?}", unsafe {
        MmIsAddressValid(0 as _)
    });
}
