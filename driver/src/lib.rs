#![no_std]
#![feature(lang_items)]
#![feature(let_else)]
#![feature(decl_macro)]
#![feature(box_syntax)]
#![allow(clippy::new_ret_no_self)]
#![allow(clippy::not_unsafe_ptr_arg_deref)]

extern crate alloc;

use crate::{
    cpuid::{CPUID_FEATURES, CPUID_HV_VENDOR},
    handlers::{cpuid, msr, rdtsc},
};
use alloc::vec;
use hypervisor::{
    debug::dbg_break,
    svm::{
        msr::{SVM_MSR_TSC, SVM_MSR_VM_HSAVE_PA},
        Hypervisor, VmExitType,
    },
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

pub mod handlers;
pub mod lang;
pub mod vm_test;

#[global_allocator]
static GLOBAL: KernelAlloc = KernelAlloc;
static LOGGER: KernelLogger = KernelLogger;

static mut HYPERVISOR: Option<Hypervisor> = None;

pub extern "system" fn driver_unload(_driver: &mut DRIVER_OBJECT) {
    if let Some(mut hv) = unsafe { HYPERVISOR.take() } {
        // This won't do anything.
        hv.devirtualize();

        core::mem::drop(hv);
    }
}

fn virtualize() -> Option<()> {
    let mut hv = Hypervisor::new(vec![])?
        .with_handler(VmExitType::Rdtsc, rdtsc::handle_rdtsc)
        .with_handler(VmExitType::Rdmsr(SVM_MSR_TSC), msr::handle_rdtsc)
        .with_handler(VmExitType::Rdmsr(SVM_MSR_VM_HSAVE_PA), msr::handle_hsave)
        .with_handler(VmExitType::Cpuid(CPUID_FEATURES), cpuid::handle_features)
        .with_handler(VmExitType::Cpuid(CPUID_HV_VENDOR), cpuid::handle_hv_vendor);

    if !hv.virtualize() {
        log::error!("Failed to virtualize processors");
        return None;
    }
    unsafe { HYPERVISOR = Some(hv) };

    Some(())
}

#[no_mangle]
pub extern "system" fn DriverEntry(driver: *mut DRIVER_OBJECT, _path: PVOID) -> NTSTATUS {
    let _ = log::set_logger(&LOGGER).map(|()| log::set_max_level(LevelFilter::Info));

    log::info!("Hello from amd_hypervisor!");

    dbg_break!();

    unsafe { (*driver).DriverUnload = Some(driver_unload) };

    if virtualize().is_none() {
        log::error!("Failed to virtualize processors");
        return STATUS_UNSUCCESSFUL;
    }

    vm_test::check_all();

    STATUS_SUCCESS
}
