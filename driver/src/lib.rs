#![no_std]
#![feature(lang_items)]
#![feature(let_else)]
#![feature(const_fmt_arguments_new)]
#![feature(const_fn_fn_ptr_basics)]
#![feature(llvm_asm)]
#![feature(untagged_unions)]
#![feature(decl_macro)]
#![feature(arbitrary_self_types)]
#![feature(const_mut_refs)]
#![feature(const_ptr_as_ref)]

use crate::debug::dbg_break;

use crate::nt::include::{KeBugCheck, MANUALLY_INITIATED_CRASH};
use crate::nt::physmem_descriptor::PhysicalMemoryDescriptor;
use crate::svm::Processors;
use core::panic::PanicInfo;
use log::{KernelLogger, LevelFilter};
use winapi::km::wdm::DRIVER_OBJECT;
use winapi::shared::{
    ntdef::{NTSTATUS, PVOID},
    ntstatus::*,
};

pub mod debug;
pub mod hook;
pub mod nt;
pub mod support;
pub mod svm;

#[no_mangle]
#[allow(bad_style)]
static _fltused: i32 = 0;

#[panic_handler]
fn panic(_info: &PanicInfo<'_>) -> ! {
    dbg_break!();

    unsafe { KeBugCheck(MANUALLY_INITIATED_CRASH) };
}

#[lang = "eh_personality"]
extern "C" fn eh_personality() {}

#[no_mangle]
extern "C" fn __CxxFrameHandler3() {}

#[global_allocator]
static GLOBAL: km_alloc::KernelAlloc = km_alloc::KernelAlloc;

static LOGGER: KernelLogger = KernelLogger;

static mut PROCESSORS: Option<Processors> = None;

#[cfg(not(feature = "mmap"))]
pub extern "system" fn driver_unload(_driver: &mut DRIVER_OBJECT) {
    // Devirtualize all processors and drop the global struct.
    //
    if let Some(mut processors) = unsafe { PROCESSORS.take() } {
        processors.devirtualize();

        core::mem::drop(processors);
    }
}

fn virtualize_system() -> Option<()> {
    let Some(mut processors) = Processors::new() else {
        log::info!("Failed to create processors");
        return None;
    };

    if !processors.virtualize() {
        log::error!("Failed to virtualize processors");
    }

    // Save the processors for later use
    //
    unsafe { PROCESSORS = Some(processors) };

    // TODO: Initialize hook here

    Some(())
}

#[no_mangle]
pub extern "system" fn DriverEntry(driver: *mut DRIVER_OBJECT, _path: PVOID) -> NTSTATUS {
    let _ = log::set_logger(&LOGGER).map(|()| log::set_max_level(LevelFilter::Trace));

    // TODO: Set this up.
    // com_logger::builder()
    //     .base(0x3E8) // Use COM3 port
    //     .filter(LevelFilter::Trace) // Print debug log
    //     .setup();

    log::info!("Hello from amd_hypervisor!");

    dbg_break!();

    log::info!("{:?}", PhysicalMemoryDescriptor::new());

    // Register `driver_unload` so we can devirtualize the processor later
    //
    cfg_if::cfg_if! {
        if #[cfg(feature = "mmap")] {
            let _ = driver;

            extern "system" fn system_thread(_context: *mut u64) {
                log::info!("System thread started");

                virtualize_system();
            }

            let mut handle = MaybeUninit::uninit();
            unsafe {
                PsCreateSystemThread(
                    handle.as_mut_ptr() as _,
                    winapi::km::wdm::GENERIC_ALL,
                    0 as _,
                    0 as _,
                    0 as _,
                    system_thread as *const (),
                    0 as _,
                )
            };

            STATUS_SUCCESS
        } else {
            log::info!("Registering driver unload routine");
            unsafe { (*driver).DriverUnload = Some(driver_unload) };

            if virtualize_system().is_some() {
                STATUS_SUCCESS
            } else {
                STATUS_UNSUCCESSFUL
            }
        }
    }
}

#[cfg(feature = "stub")]
#[no_mangle]
pub extern "system" fn _DllMainCRTStartup() {}
