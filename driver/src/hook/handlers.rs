use crate::nt::inline_hook::InlineHook;
use crate::nt::ptr::Pointer;
use winapi::shared::ntdef::NTSTATUS;

pub static mut ZWQSI_ORIGINAL: Option<Pointer<InlineHook>> = None;
pub fn zw_query_system_information(
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
            ZWQSI_ORIGINAL.as_ref().unwrap().as_ptr(),
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
            EAPWT_ORIGINAL.as_ref().unwrap().as_ptr(),
        )
    };
    fn_ptr(pool_tag, number_of_bytes, tag)
}
