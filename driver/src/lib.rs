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
#![feature(const_trait_impl)]
#![allow(clippy::new_ret_no_self)]
#![feature(int_abs_diff)]

extern crate alloc;

#[macro_use] extern crate static_assertions;

use crate::{
    debug::dbg_break,
    hook::{handlers, testing, Hook, HookType},
    nt::{
        include::{KeBugCheck, MANUALLY_INITIATED_CRASH},
        inline_hook::FunctionHook,
        physmem_descriptor::PhysicalMemoryDescriptor,
        ptr::Pointer,
    },
    svm::Processors,
};
use alloc::{vec, vec::Vec};
use log::{KernelLogger, LevelFilter};
use winapi::{
    km::wdm::DRIVER_OBJECT,
    shared::{
        ntdef::{NTSTATUS, PVOID},
        ntstatus::*,
    },
};

pub mod debug;
pub mod hook;
pub mod lang;
pub mod nt;
pub mod support;
pub mod svm;

#[global_allocator]
static GLOBAL: km_alloc::KernelAlloc = km_alloc::KernelAlloc;
static LOGGER: KernelLogger = KernelLogger;

static mut PROCESSORS: Option<Processors> = None;

fn init_hooks() -> Option<Vec<Hook>> {
    // ZwQuerySystemInformation
    //
    let zwqsi_hook = Hook::hook_function(
        "ZwQuerySystemInformation",
        handlers::zw_query_system_information as *const (),
    )?;
    unsafe {
        handlers::ZWQSI_ORIGINAL = match zwqsi_hook.hook_type {
            HookType::Function { ref inline_hook } => Pointer::new(inline_hook.as_ptr()),
            HookType::Page => None,
        };
    }

    // // ExAllocatePoolWithTag
    // //
    // let eapwt_hook = Hook::hook_function(
    //     "ExAllocatePoolWithTag",
    //     handlers::ex_allocate_pool_with_tag as *const (),
    // )?;
    // unsafe {
    //     handlers::EAPWT_ORIGINAL = match eapwt_hook.hook_type {
    //         HookType::Function { ref inline_hook } =>
    // Pointer::new(inline_hook.as_ptr()),         HookType::Page =>
    // unreachable!(),     };
    // }

    // // MmIsAddressValid
    // //
    let mmiav_hook = Hook::hook_function(
        "MmIsAddressValid",
        handlers::mm_is_address_valid as *const (),
    )?;
    unsafe {
        handlers::MMIAV_ORIGINAL = match mmiav_hook.hook_type {
            HookType::Function { ref inline_hook } => Pointer::new(inline_hook.as_ptr()),
            HookType::Page => unreachable!(),
        };
    }

    let hook = Hook::hook_function_ptr(
        unsafe { testing::SHELLCODE_PA.as_ref().unwrap().va() as u64 },
        testing::hook_handler as *const (),
    )?;

    // FIXME: Currently only 1 hook is supported

    Some(vec![zwqsi_hook, mmiav_hook, hook])
}

fn virtualize_system() -> Option<()> {
    let hooks = init_hooks()?;
    let Some(mut processors) = Processors::new(hooks) else {
        log::info!("Failed to create processors");
        return None;
    };

    if !processors.virtualize() {
        log::error!("Failed to virtualize processors");
    }

    // Save the processors for later use
    //
    unsafe { PROCESSORS = Some(processors) };

    Some(())
}

#[cfg(not(feature = "mmap"))]
pub extern "system" fn driver_unload(_driver: &mut DRIVER_OBJECT) {
    // Devirtualize all processors and drop the global struct.
    //
    if let Some(mut processors) = unsafe { PROCESSORS.take() } {
        processors.devirtualize();

        core::mem::drop(processors);
    }
}

#[no_mangle]
pub extern "system" fn DriverEntry(driver: *mut DRIVER_OBJECT, _path: PVOID) -> NTSTATUS {
    let _ = log::set_logger(&LOGGER).map(|()| log::set_max_level(LevelFilter::Info));

    log::info!("Hello from amd_hypervisor!");

    dbg_break!();

    // Initialize the hook testing
    //

    testing::init();
    testing::print_shellcode();
    testing::call_shellcode();
    testing::print_shellcode();

    // Print physical memory pages
    //

    let desc = PhysicalMemoryDescriptor::new();

    log::info!("Physical memory descriptors: {:x?}", desc);
    log::info!("Found {:#x?} pages", desc.page_count());
    log::info!("Found {}gb of physical memory", desc.total_size_in_gb());

    // Virtualize the system
    //
    cfg_if::cfg_if! {
        if #[cfg(feature = "mmap")] {
            let _ = driver;

            extern "system" fn system_thread(_context: *mut u64) {
                log::info!("System thread started");

                virtualize_system();
            }

            let mut handle = core::mem::MaybeUninit::uninit();
            unsafe {
                crate::nt::include::PsCreateSystemThread(
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
            #[allow(clippy::not_unsafe_ptr_arg_deref)]
            unsafe { (*driver).DriverUnload = Some(driver_unload) };

            let status = if virtualize_system().is_some() {
                STATUS_SUCCESS
            } else {
                STATUS_UNSUCCESSFUL
            };

            // Call the hook again after initialization
            //
            testing::print_shellcode();
            testing::call_shellcode();
            testing::print_shellcode();

            handlers::test_hooks();

            status
        }
    }
}
