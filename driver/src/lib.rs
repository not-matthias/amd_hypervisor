#![no_std]
#![feature(lang_items)]
#![feature(let_else)]
#![feature(decl_macro)]
#![feature(box_syntax)]
#![allow(clippy::new_ret_no_self)]
#![allow(clippy::not_unsafe_ptr_arg_deref)]

extern crate alloc;

use hypervisor::{
    debug::dbg_break,
    svm::Hypervisor,
    utils::{alloc::KernelAlloc, logger::KernelLogger},
};
use log::LevelFilter;
use winapi::{
    km::wdm::DRIVER_OBJECT,
    shared::{
        ntdef::{NTSTATUS, PVOID},
        ntstatus::*,
    },
};

pub mod lang;
pub mod vm_test;

#[global_allocator]
static GLOBAL: KernelAlloc = KernelAlloc;
static LOGGER: KernelLogger = KernelLogger;

static mut HYPERVISOR: Option<Hypervisor> = None;

pub extern "system" fn driver_unload(_driver: &mut DRIVER_OBJECT) {
    if let Some(mut hv) = unsafe { HYPERVISOR.take() } {
        hv.devirtualize();

        core::mem::drop(hv);
    }
}

#[no_mangle]
pub extern "system" fn DriverEntry(driver: *mut DRIVER_OBJECT, _path: PVOID) -> NTSTATUS {
    let _ = log::set_logger(&LOGGER).map(|()| log::set_max_level(LevelFilter::Info));

    log::info!("Hello from amd_hypervisor!");

    dbg_break!();

    unsafe { (*driver).DriverUnload = Some(driver_unload) };

    // // Virtualize the system
    // //
    // let Some(mut hv) = Hypervisor::new(hooks) else {
    //     log::info!("Failed to create processors");
    //     return None;
    // };
    //
    // if !hv.virtualize() {
    //     log::error!("Failed to virtualize processors");
    // }
    //
    // unsafe { HYPERVISOR = Some(hv) };

    vm_test::check_all();

    STATUS_SUCCESS
}
