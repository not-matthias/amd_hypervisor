#![no_std]
#![feature(lang_items)]
#![feature(let_else)]
#![feature(decl_macro)]
#![feature(box_syntax)]
#![feature(new_uninit)]
#![allow(clippy::new_ret_no_self)]
#![allow(clippy::not_unsafe_ptr_arg_deref)]

extern crate alloc;

use crate::{
    cpuid::CPUID_FEATURES,
    handlers::{bp, cpuid, npf, rdtsc},
};
use alloc::vec;
use hypervisor::{
    debug::dbg_break,
    hook::HookManager,
    svm::{Hypervisor, VmExitType},
};
use kernel_alloc::KernelAlloc;
use kernel_log::KernelLogger;
use log::LevelFilter;
use winapi::{
    km::wdm::DRIVER_OBJECT,
    shared::{
        ntdef::{NTSTATUS, PVOID},
        ntstatus::*,
    },
};

pub mod handlers;
pub mod hook;
pub mod lang;

#[global_allocator]
static GLOBAL: KernelAlloc = KernelAlloc;

static mut HOOK_MANAGER: Option<HookManager> = None;
static mut HYPERVISOR: Option<Hypervisor> = None;

pub extern "system" fn driver_unload(_driver: &mut DRIVER_OBJECT) {
    if let Some(mut hv) = unsafe { HYPERVISOR.take() } {
        hv.devirtualize();

        core::mem::drop(hv);
    }
}

fn virtualize() -> Option<()> {
    let mut hv = Hypervisor::new()?
        .with_handler(VmExitType::Rdtsc, rdtsc::handle_rdtsc)
        .with_handler(VmExitType::Cpuid(CPUID_FEATURES), cpuid::handle_features)
        .with_handler(VmExitType::Breakpoint, bp::handle_bp_exception)
        .with_handler(VmExitType::NestedPageFault, npf::handle_npf);

    if !hv.virtualize() {
        log::error!("Failed to virtualize processors");
        return None;
    }
    unsafe { HYPERVISOR = Some(hv) };

    // Initialize the hook manager
    //
    unsafe { HOOK_MANAGER = Some(HookManager::new(vec![])) };

    Some(())
}

#[no_mangle]
pub extern "system" fn DriverEntry(driver: *mut DRIVER_OBJECT, _path: PVOID) -> NTSTATUS {
    KernelLogger::init(LevelFilter::Info).unwrap();

    log::info!("Hello from amd_hypervisor!");

    dbg_break!();

    unsafe { (*driver).DriverUnload = Some(driver_unload) };

    if virtualize().is_none() {
        log::error!("Failed to virtualize processors");
        return STATUS_UNSUCCESSFUL;
    }

    STATUS_SUCCESS
}
