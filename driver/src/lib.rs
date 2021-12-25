#![no_std]
#![feature(lang_items)]
#![feature(let_else)]

use ::nt::include::IRP_MJ_DEVICE_CONTROL;
use core::panic::PanicInfo;
use km_alloc::KernelAlloc;
use log::{KernelLogger, LevelFilter};
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

pub extern "C" fn driver_unload(_driver: PDRIVER_OBJECT) {
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
    unsafe {
        ((*driver)
            .MajorFunction
            .as_mut_ptr()
            .add(IRP_MJ_DEVICE_CONTROL) as *mut u64)
            .write_volatile(driver_unload as *const () as _)
    };

    // Check whether svm is supported
    //
    if !support::is_svm_supported() {
        log::error!("SVM is not supported");
        return STATUS_UNSUCCESSFUL;
    } else {
        log::info!("SVM is supported");
    }

    // Virtualize processors
    //
    log::info!("Virtualizing processors");

    STATUS_SUCCESS
}
