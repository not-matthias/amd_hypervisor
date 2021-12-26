#![no_std]
#![feature(lang_items)]
#![feature(let_else)]
#![feature(const_fmt_arguments_new)]
#![feature(const_fn_fn_ptr_basics)]

use crate::svm::Processors;
use core::panic::PanicInfo;

use km_alloc::KernelAlloc;
use log::{KernelLogger, LevelFilter};
use winapi::km::wdm::DRIVER_OBJECT;
use winapi::km::wdm::PDRIVER_OBJECT;
use winapi::shared::{
    ntdef::{NTSTATUS, PVOID},
    ntstatus::*,
};

pub mod nt;
pub mod support;
pub mod svm;

#[no_mangle]
#[allow(bad_style)]
static _fltused: i32 = 0;

#[panic_handler]
fn panic(info: &PanicInfo<'_>) -> ! {
    log::error!("Panic: {}", info);

    loop {}
}

#[lang = "eh_personality"]
extern "C" fn eh_personality() {}

#[no_mangle]
extern "C" fn __CxxFrameHandler3() {}

#[global_allocator]
static GLOBAL: KernelAlloc = KernelAlloc;

static LOGGER: KernelLogger = KernelLogger;

pub extern "system" fn driver_unload(_driver: &mut DRIVER_OBJECT) {
    // Devirtualize all processors
    //
    // TODO: Implement
}

#[no_mangle]
pub extern "system" fn DriverEntry(driver: PDRIVER_OBJECT, _path: PVOID) -> NTSTATUS {
    let _ = log::set_logger(&LOGGER).map(|()| log::set_max_level(LevelFilter::Trace));

    log::info!("Hello from amd_hypervisor!");

    // Register `driver_unload` so we can devirtualize the processor later
    //
    log::info!("Registering driver unload routine");
    unsafe { (*driver).DriverUnload = Some(driver_unload) };

    // Virtualize processors
    //
    let Some(processors) = Processors::new() else {
        log::info!("Failed to create processors");
        return STATUS_UNSUCCESSFUL;
    };

    if !processors.virtualize() {
        log::error!("Failed to virtualize processors");
    }

    // TODO: Devirtualize and free memory when failing (and when unloading)

    STATUS_SUCCESS
}
