//! This modules contains some code to be able to test the hooking system easier.

use crate::nt::addresses::PhysicalAddress;
use crate::nt::memory::AllocatedMemory;

pub static mut ALLOCATED_MEMORY: Option<AllocatedMemory<u8>> = None;

/// The physical address of the allocated page.
pub static mut SHELLCODE_PA: Option<PhysicalAddress> = None;

pub fn init() -> Option<()> {
    // Allocate the memory
    //
    let memory = AllocatedMemory::<u8>::alloc(0x1000)?;

    // Write our shellcode to the page
    //
    // mov rax, 0x42
    // ret
    //
    let shellcode = [0x90, 0x48, 0xC7, 0xC0, 0x84, 0x00, 0x00, 0x00, 0xC3];
    unsafe { core::ptr::copy(shellcode.as_ptr(), memory.as_ptr(), shellcode.len()) };

    // Set the globals
    //
    unsafe {
        SHELLCODE_PA = Some(PhysicalAddress::from_va(memory.as_ptr() as *mut u64 as u64));
        ALLOCATED_MEMORY = Some(memory);
    }

    Some(())
}

pub fn call_shellcode() {
    type ShellcodeFn = extern "C" fn() -> u64;

    let fn_ptr = unsafe { ALLOCATED_MEMORY.as_ref().unwrap().as_ptr() as *mut u64 };
    log::info!("Calling shellcode at {:p}", fn_ptr);

    let fn_ptr = unsafe { core::mem::transmute::<_, ShellcodeFn>(fn_ptr) };

    log::info!("Return value: {:x}", fn_ptr());
}

pub fn print_shellcode() {
    let fn_ptr = unsafe { ALLOCATED_MEMORY.as_ref().unwrap().as_ptr() as *mut u8 };

    log::info!("Printing shellcode at {:p}", fn_ptr);
    for i in 0..9 {
        log::info!("{:x}", unsafe { *fn_ptr.add(i) });
    }
}
