//! This modules contains some code to be able to test the hooking system easier.

use crate::nt::addresses::PhysicalAddress;
use crate::nt::memory::AllocatedMemory;
use crate::FunctionHook;

pub static mut ALLOCATED_MEMORY: Option<AllocatedMemory<u8>> = None;

/// The physical address of the allocated page.
pub static mut SHELLCODE_PA: Option<PhysicalAddress> = None;

pub fn init() -> Option<()> {
    // Allocate the memory
    //
    let memory = AllocatedMemory::<u8>::alloc(0x1000)?;

    // Write our shellcode to the page
    //
    // ```
    // add_two:
    // add rcx, 0x2
    // mov rax, rcx
    // ret
    // ```
    let shellcode = [
        0x90, 0x90, 0x90, 0x90, 0x90, 0x90, 0x90, 0x90, 0x90, 0x90, 0x90, 0x90, 0x90, 0x90, 0x90,
        0x48, 0x83, 0xC1, 0x02, 0x48, 0x89, 0xC8, 0xC3,
    ];

    // Copy to page start
    //
    unsafe { core::ptr::copy(shellcode.as_ptr(), memory.as_ptr(), shellcode.len()) };

    // Copy to page middle
    //
    unsafe {
        core::ptr::copy(
            shellcode.as_ptr(),
            memory.as_ptr().add(0x500),
            shellcode.len(),
        )
    };

    // Set the globals
    //
    unsafe {
        SHELLCODE_PA = Some(PhysicalAddress::from_va(memory.as_ptr() as *mut u64 as u64));
        ALLOCATED_MEMORY = Some(memory);
    }

    Some(())
}

pub fn call_shellcode() {
    type ShellcodeFn = extern "C" fn(u64) -> u64;

    let fn_ptr = unsafe {
        core::mem::transmute::<_, ShellcodeFn>(
            ALLOCATED_MEMORY.as_ref().unwrap().as_ptr() as *mut u64
        )
    };
    log::info!("[page] add_two(42): {}", fn_ptr(42));

    let fn_ptr = unsafe {
        core::mem::transmute::<_, ShellcodeFn>(
            (ALLOCATED_MEMORY.as_ref().unwrap().as_ptr().offset(0x500)) as *mut u64,
        )
    };
    log::info!("[page+0x500] add_two(42): {}", fn_ptr(42));
}

pub fn print_shellcode() {
    let fn_ptr = unsafe { ALLOCATED_MEMORY.as_ref().unwrap().as_ptr() as *mut u8 };
    let slice = unsafe { core::slice::from_raw_parts(fn_ptr, 15) };

    log::info!("Printing shellcode at {:x?}", slice);
}

// ============================================================================

pub static mut HOOK: Option<AllocatedMemory<FunctionHook>> = None;

// pub fn setup_hook() {
//     let hook = unsafe {
//         InlineHook::new(
//             ALLOCATED_MEMORY.as_ref().unwrap().as_ptr() as _,
//             hook_handler as _,
//         )
//     }
//     .unwrap();
//
//     hook.enable();
//
//     unsafe { HOOK = Some(hook) };
// }

pub fn hook_handler(a: u64) -> u64 {
    log::info!("hook handler called");

    // let trampoline = unsafe { crate::hook::HOOK.as_ref().unwrap().trampoline_address() };
    // let trampoline_fn = unsafe { core::mem::transmute::<_, extern "C" fn(u64) -> u64>(trampoline) };
    //
    // trampoline_fn(a + 2)

    a
}
