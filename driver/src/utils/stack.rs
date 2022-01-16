use crate::utils::nt::RtlCaptureContext;
use alloc::vec::Vec;
use core::mem::MaybeUninit;
use nt::include::MmIsAddressValid;
use winapi::um::winnt::{RtlLookupFunctionEntry, RtlVirtualUnwind, CONTEXT, UNW_FLAG_NHANDLER};

fn get_context() -> CONTEXT {
    let mut context = MaybeUninit::<CONTEXT>::uninit();
    unsafe {
        RtlCaptureContext(context.as_mut_ptr());
        context.assume_init()
    }
}

/// Finds the return address by walking the stack.
///
/// ## References
/// - [`StackTrace64`](http://www.nynaeve.net/Code/StackWalk64.cpp)
/// - [`EAThread - GetCallstack`](https://github.com/electronicarts/EAThread/blob/master/source/pc/eathread_callstack_win64.cpp#L264)
/// - [`RtlVirtualUnwind`](https://docs.rs/winapi/latest/winapi/um/winnt/fn.RtlVirtualUnwind.html)
/// - [`RtlLookupFunctionEntry`](https://docs.rs/winapi/latest/winapi/um/winnt/fn.RtlLookupFunctionEntry.html)
pub fn return_address_by_rip(rip: u64) -> Option<u64> {
    // Try to find `RUNTIME_FUNCTION` via `RtlLookupFunctionEntry`. See: https://github.com/electronicarts/EAThread/blob/master/source/pc/eathread_callstack_win64.cpp#L323
    //
    let mut image_base = 0;
    let runtime_function =
        unsafe { RtlLookupFunctionEntry(rip, &mut image_base as *mut _, core::ptr::null_mut()) };

    if runtime_function.is_null() {
        log::warn!("RtlLookupFunctionEntry failed");
        return None;
    }

    // Create a new context, which will store the return address (and other
    // registers).
    //
    let mut new_context = MaybeUninit::<CONTEXT>::uninit();
    unsafe { RtlCaptureContext(new_context.as_mut_ptr()) };

    // Unwind the stack using `RtlVirtualUnwind`.
    //
    let mut handler_data = MaybeUninit::uninit();
    let mut establisher_frame = [0u64; 2];
    unsafe {
        RtlVirtualUnwind(
            UNW_FLAG_NHANDLER,
            image_base,
            rip,
            runtime_function,
            new_context.as_mut_ptr(),
            handler_data.as_mut_ptr(),
            establisher_frame.as_mut_ptr(),
            core::ptr::null_mut(),
        )
    };

    Some(unsafe { new_context.assume_init().Rip })
}

#[inline(always)]
pub fn current_return_address() -> Option<u64> {
    let context = get_context();
    return_address_by_rip(context.Rip)
}

/// Returns the return address that is the first element on the stack.
pub fn top_return_address(rsp: u64) -> Option<u64> {
    let stack_ptr = rsp as *const u64;
    if stack_ptr.is_null() || unsafe { !MmIsAddressValid(stack_ptr as *mut _) } {
        log::warn!("Invalid stack pointer: {:x}", rsp);
        return None;
    }

    let return_address = unsafe { stack_ptr.read_volatile() };
    if return_address > 0x7FFF_FFFF_FFFF {
        Some(return_address)
    } else {
        None
    }
}

/// Tries to find the return address based on the stack pointer.
///
/// ## References
///
/// - https://hikalium.github.io/opv86/?q=call
/// - https://www.felixcloutier.com/x86/call
pub fn find_return_addresses(rsp: u64) -> Option<Vec<u64>> {
    const MAX_DEPTH: usize = 15;

    let stack = unsafe { core::slice::from_raw_parts(rsp as *const u64, MAX_DEPTH) };
    log::info!("stack: {:x?}", stack);

    let mut return_addresses = Vec::new();
    for item in stack {
        if *item > 0x7FFF_FFFF_FFFF {
            return_addresses.push(*item);
        }
    }

    Some(return_addresses)
}
