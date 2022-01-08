use crate::nt::include::RtlCaptureContext;
use core::mem::MaybeUninit;
use nt::include::MmIsAddressValid;
use winapi::um::winnt::RtlLookupFunctionEntry;
use winapi::um::winnt::RtlVirtualUnwind;
use winapi::um::winnt::CONTEXT;
use winapi::um::winnt::UNW_FLAG_NHANDLER;

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

    // Create a new context, which will store the return address (and other registers).
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

/// Tries to find the return address based on the stack pointer.
///
/// ## References
///
/// - https://hikalium.github.io/opv86/?q=call
/// - https://www.felixcloutier.com/x86/call
///
#[deprecated(note = "Use `return_address_by_rip` instead.")]
pub fn find_return_address_manual(rsp: u64) -> Option<u64> {
    const MAX_DEPTH: usize = 15;

    for i in 0..MAX_DEPTH {
        let stack_ptr = unsafe { (rsp as *mut u64).add(i) };
        if unsafe { !MmIsAddressValid(stack_ptr as _) } {
            continue;
        }

        let ret_addr = unsafe { stack_ptr.read_volatile() };
        if ret_addr < 0x7FFF_FFFF_FFFF || unsafe { !MmIsAddressValid(ret_addr as _) } {
            continue;
        }

        let valid_opcode = |addr, opcode, opcode_size: isize| {
            let opcode_ptr = unsafe { (addr as *mut u8).offset(-opcode_size) };
            if unsafe { !MmIsAddressValid(opcode_ptr as _) } {
                return None;
            }

            let opcode_value = unsafe { (addr as *mut u8).offset(-opcode_size).read_volatile() };
            if opcode_value == opcode {
                Some(addr)
            } else {
                None
            }
        };

        // Call near, relative, displacement relative to next instruction. 32-bit displacement sign extended to 64-bits in 64-bit mode.
        //
        const REL_NEAR_OPCODE: u8 = 0xE8;
        const REL_NEAR_SIZE: isize = 5;

        if let Some(addr) = valid_opcode(ret_addr, REL_NEAR_OPCODE, REL_NEAR_SIZE) {
            return Some(addr);
        }

        // Call near, absolute indirect, address given in r/m64.
        //
        const CALL_NEAR_ABS_IND_OPCODE: u8 = 0xFF;
        const CALL_NEAR_ABS_IND_SIZE: isize = 2;

        if let Some(addr) = valid_opcode(ret_addr, CALL_NEAR_ABS_IND_OPCODE, CALL_NEAR_ABS_IND_SIZE)
        {
            return Some(addr);
        }

        // Call far, absolute indirect address given in m16:16.
        //
        // Example: `call    qword ptr [rax+28h]` (opcodes: `FF 50 28`)
        //
        const CALL_FAR_ABS_IND_OPCODE: u8 = 0xFF;
        const CALL_FAR_ABS_IND_SIZE: isize = 3;

        if let Some(addr) = valid_opcode(ret_addr, CALL_FAR_ABS_IND_OPCODE, CALL_FAR_ABS_IND_SIZE) {
            return Some(addr);
        }

        // TODO: Find out which one this is
        //
        // fffff60d`4cb71a7a 48ff1597ff3300  call    qword ptr [win32kfull!_imp_ZwAssociateWaitCompletionPacket (fffff60d`4ceb1a18)]
        // fffff60d`4cb71a7a 48ff1597ff3300  call    qword ptr [win32kfull!_imp_ZwAssociateWaitCompletionPacket (fffff60d`4ceb1a18)]
        //
        const CUSTOM_OPCODE: u8 = 0x48;
        const CUSTOM_SIZE: isize = 7;

        if let Some(addr) = valid_opcode(ret_addr, CUSTOM_OPCODE, CUSTOM_SIZE) {
            return Some(addr);
        }
    }

    None
}
