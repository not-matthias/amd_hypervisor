#![no_std]
#![feature(lang_items)]
#![feature(let_else)]
#![feature(decl_macro)]
#![feature(box_syntax)]
#![feature(new_uninit)]
#![feature(link_llvm_intrinsics)]
#![allow(clippy::new_ret_no_self)]
#![allow(clippy::not_unsafe_ptr_arg_deref)]

extern crate alloc;

use crate::{
    cpuid::CPUID_FEATURES,
    handlers::{bp, cpuid, npf},
};
use alloc::vec;
use core::sync::atomic::Ordering;
use hypervisor::{
    hook::{Hook, HookManager, HookType},
    svm::{
        nested_page_table::NestedPageTable, utils::paging::AccessType, vmexit::rdtsc, Hypervisor,
        VmExitType,
    },
    utils::debug::dbg_break,
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
    // Initialize the hook and hook manager
    //
    let hook = Hook::hook_function("MmIsAddressValid", hook::mm_is_address_valid as *const ())?;
    if let HookType::Function { ref inline_hook } = hook.hook_type {
        hook::ORIGINAL.store(inline_hook.trampoline_address(), Ordering::Relaxed);
    }
    unsafe { HOOK_MANAGER = Some(HookManager::new(vec![hook])) };

    // Create the hypervisor with some handlers. If you have handlers that are in
    // another crate, you can export an array and add them via `with_handlers`.
    //
    // I have another crate which has the handlers that harden hypervisor against
    // detection and can import them all by calling `with_handlers` once.
    //
    let mut hv = Hypervisor::builder()
        .with_handlers([
            (VmExitType::Rdtsc, rdtsc::handle_default),
            (VmExitType::Cpuid(CPUID_FEATURES), cpuid::handle_features),
            (VmExitType::Breakpoint, bp::handle_bp_exception),
            (VmExitType::NestedPageFault, npf::handle_npf),
        ])
        .primary_npt(NestedPageTable::identity_4kb(AccessType::ReadWriteExecute))
        .secondary_npt(NestedPageTable::identity_4kb(AccessType::ReadWrite))
        .build()?;

    if !hv.virtualize() {
        log::error!("Failed to virtualize processors");
        return None;
    }
    unsafe { HYPERVISOR = Some(hv) };

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
