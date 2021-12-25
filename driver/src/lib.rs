#![no_std]
#![feature(lang_items)]
#![feature(let_else)]

use core::panic::PanicInfo;
use km_alloc::KernelAlloc;
use log::{KernelLogger, LevelFilter};
use winapi::shared::{
    ntdef::{NTSTATUS, PVOID},
    ntstatus::*,
};

pub mod ioctl;
pub mod support;

#[no_mangle]
#[allow(bad_style)]
static _fltused: i32 = 0;

#[panic_handler]
const fn panic(_info: &PanicInfo<'_>) -> ! {
    loop {}
}

#[lang = "eh_personality"]
extern "C" fn eh_personality() {}

#[no_mangle]
extern "C" fn __CxxFrameHandler3() {}

#[global_allocator]
static GLOBAL: KernelAlloc = KernelAlloc;

static LOGGER: KernelLogger = KernelLogger;

#[no_mangle]
pub extern "system" fn DriverEntry(_driver: PVOID, _path: PVOID) -> NTSTATUS {
    let _ = log::set_logger(&LOGGER).map(|()| log::set_max_level(LevelFilter::Trace));

    log::info!("Hello from amd_hypervisor!");

    // Check whether svm is supported
    //
    if !support::is_svm_supported() {
        log::error!("SVM is not supported");
        return STATUS_UNSUCCESSFUL;
    } else {
        log::info!("SVM is supported");
    }

    // // Hook major function
    // //
    // hook::device_control::hook_major_function(
    //     obfstr!(L "\\Driver\\Null"),
    //     hook_handler as *const (),
    // );

    STATUS_SUCCESS
}
