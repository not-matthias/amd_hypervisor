use crate::nt::inline_hook::InlineHook;
use crate::nt::ptr::Pointer;
use nt::include::MmIsAddressValid;
use winapi::shared::ntdef::NTSTATUS;

pub static mut ZWQSI_ORIGINAL: Option<Pointer<InlineHook>> = None;
pub extern "system" fn zw_query_system_information(
    system_information_class: u32,
    system_information: u64,
    system_information_length: u32,
    return_length: u32,
) -> NTSTATUS {
    log::info!(
        "Called zw_query_system_information({:x}, {:x}, {:x}, {:x})",
        system_information_class,
        system_information,
        system_information_length,
        return_length
    );

    // Call original
    //
    log::info!("Calling original.");
    let fn_ptr = unsafe {
        core::mem::transmute::<_, fn(u32, u64, u32, u32) -> NTSTATUS>(
            ZWQSI_ORIGINAL.as_ref().unwrap().trampoline_address(),
        )
    };
    fn_ptr(
        system_information_class,
        system_information,
        system_information_length,
        return_length,
    )
}

pub static mut EAPWT_ORIGINAL: Option<Pointer<InlineHook>> = None;
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

// Currently not supported because of the minimalistic InlineHook implementation.
pub static mut MMIAV_ORIGINAL: Option<Pointer<InlineHook>> = None;
pub fn mm_is_address_valid(ptr: u64) -> bool {
    log::info!("Called mm_is_address_valid({:x})", ptr);

    // Call original
    //
    let fn_ptr = unsafe {
        core::mem::transmute::<_, fn(u64) -> bool>(
            MMIAV_ORIGINAL.as_ref().unwrap().trampoline_address(),
        )
    };
    log::info!("Calling original: {:x}", fn_ptr as u64);

    fn_ptr(ptr)
}

// This can't be in the same page as the hook handler.
#[link_section = ".custom$test_hooks"]
#[inline(never)]
pub fn test_hooks() {
    // Test zw_query_system_information
    //
    // log::info!("Testing zw_query_system_information.");
    // let status = unsafe {
    //     ZwQuerySystemInformation(
    //         SYSTEM_INFORMATION_CLASS::SystemProcessInformation,
    //         0x0 as _,
    //         0x0,
    //         0x0 as _,
    //     )
    // };
    // log::info!("zw_query_system_information returned {:x}.", status);

    // Test ex_allocate_pool_with_tag
    //
    // log::info!("EAPWT_ORIGINAL: {:?}", unsafe {
    //     EAPWT_ORIGINAL.as_ref().unwrap().as_ptr()
    // });
    // let ptr = unsafe { ExAllocatePoolWithTag(NonPagedPool as _, 0x20, 0xABCD) };
    // log::info!("ex_allocate_pool_with_tag returned {:x}.", ptr);
    //
    // dbg_break!();
    //
    // unsafe { ExFreePool(ptr as _) };

    // Test MmIsAddressValid
    //
    log::info!("Is address valid: {:?}", unsafe {
        MmIsAddressValid(0 as _)
    });
}
