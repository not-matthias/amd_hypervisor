#![feature(llvm_asm)]

use std::arch::global_asm;
use x86::bits64::registers::rsp;

global_asm!(include_str!("vmlaunch.asm"));

#[no_mangle]
pub unsafe extern "C" fn handle_vmexit() {
    let rcx: u64;
    unsafe {
        llvm_asm!("mov %rcx, $0" : "=r" (rcx) ::);
    }

    println!("RSP is: {:x}", rcx);
}

extern "C" {
    pub fn launch_vm(host_rsp: u64);
}

fn main() {
    unsafe {
        launch_vm(0x42);
    }
}