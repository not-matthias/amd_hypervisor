#![allow(dead_code)]
#![allow(unused)]

extern crate alloc;

use crate::hook_testing::hook_handler;
use crate::nt::addresses::PhysicalAddress;
use crate::nt::include::{assert_paged_code, RtlCopyMemory};
use crate::nt::inline_hook::InlineHook;
use crate::nt::memory::AllocatedMemory;
use crate::svm::data::nested_page_table::NestedPageTable;
use crate::svm::paging::AccessType;
use alloc::string::String;
use alloc::vec::Vec;
use nt::include::SYSTEM_INFORMATION_CLASS;
use nt::kernel::get_system_routine_address;
use x86::bits64::paging::{PAddr, BASE_PAGE_SIZE};
use x86::current::paging::VAddr;
use x86_64::instructions::interrupts::without_interrupts;

// TODO: Implement these different hook types
//
// There's different hooking types:
// - Hide memory: Simply writes your own executable code over another and don't care about the original.
// - Patch function: Overwrite a function without calling the original function.
// - Hook function: Overwrite a function, but call the original function. TODO: Implement this
//

// TODO: Convert this to an enum for the different hook types
pub struct Hook {
    pub original_va: u64,
    pub original_pa: PhysicalAddress,

    pub hook_va: u64,
    pub hook_pa: PhysicalAddress,

    // TODO: Unused for now
    pub handler: Option<InlineHook>,

    pub page: AllocatedMemory<u8>,
}

pub static mut HOOK: Option<AllocatedMemory<InlineHook>> = None;

impl Hook {
    fn copy_page(address: u64) -> Option<AllocatedMemory<u8>> {
        // Why does this crash because of a page fault? See: https://docs.microsoft.com/en-us/windows-hardware/drivers/kernel/when-should-code-and-data-be-pageable-

        log::info!("Creating a copy of the page at {:#x}", address);

        let page_address = PAddr::from(address).align_down_to_base_page();
        if page_address.is_zero() {
            log::error!("Invalid address: {:#x}", address);
            return None;
        }
        let page = AllocatedMemory::<u8>::alloc(BASE_PAGE_SIZE)?;

        log::info!("Page address: {:#x}", page_address);

        assert_paged_code!();

        without_interrupts(|| {
            unsafe {
                RtlCopyMemory(
                    page.as_ptr() as _,
                    page_address.as_u64() as *mut u64,
                    BASE_PAGE_SIZE,
                )
            };
        });

        log::info!("After copying the memory.");

        Some(page)
    }

    pub fn from_address(address: u64) -> Option<Self> {
        let physical_address = PhysicalAddress::from_va(address);
        let page = Self::copy_page(address)?;

        let hook_va = page.as_ptr() as *mut u64 as u64;
        let hook_pa = PhysicalAddress::from_va(hook_va);

        Some(Self {
            original_va: address,
            original_pa: physical_address,
            hook_va,
            hook_pa,
            handler: None,
            page,
        })
    }

    pub fn from_address_with_handler(address: u64, handler: *const ()) -> Option<Self> {
        let physical_address = PhysicalAddress::from_va(address);
        let page = Self::copy_page(address)?;

        let hook_va = page.as_ptr() as *mut u64 as u64;
        let hook_pa = PhysicalAddress::from_va(hook_va);

        // Find the offset from the page base to the function.
        let fn_offset = VAddr::from_u64(address).base_page_offset();
        let hook_fn_ptr = hook_va + fn_offset;
        log::info!("Placing inline hook at {:#x}", hook_fn_ptr);

        unsafe {
            HOOK = Some(
                InlineHook::new(hook_fn_ptr, hook_handler as *const ())
                    .expect("Failed to create inline hook"),
            );
        }

        Some(Self {
            original_va: address,
            original_pa: physical_address,
            hook_va,
            hook_pa,
            handler: None,
            page,
        })
    }

    pub fn new(name: &str, handler: *const ()) -> Option<Self> {
        let address = get_system_routine_address(name)? as u64;
        log::info!("Found address of {}: {:#x}", &name, address);

        let physical_address = PhysicalAddress::from_va(address);
        log::info!("Physical address: {:#x}", physical_address.as_u64());

        let page = Self::copy_page(address)?;
        let hook_va = page.as_ptr() as *mut u64 as u64;
        let hook_pa = PhysicalAddress::from_va(hook_va);

        //

        Some(Self {
            original_va: address,
            original_pa: physical_address,
            hook_va,
            hook_pa,
            handler: None,
            page,
        })
    }
}

pub struct HookedNpt {
    pub npt: AllocatedMemory<NestedPageTable>,

    // TODO: Can we remove these useless allocations?
    pub hooks: Vec<Hook>,
}

impl HookedNpt {
    pub fn new() -> Option<AllocatedMemory<Self>> {
        let mut hooked_npt = AllocatedMemory::<Self>::alloc(core::mem::size_of::<Self>())?;

        hooked_npt.npt = NestedPageTable::identity_2mb()?;
        hooked_npt.hooks = Vec::new();

        Some(hooked_npt)
    }

    /// Hooks the specified function.
    ///
    /// ## Parameters
    ///
    /// - `function`: The name of the function to hook.
    /// - `handler`: The function that should be called from the hook.
    ///
    pub fn hook(&mut self, function: &str, handler: *const ()) -> Option<()> {
        let hook = Hook::new(function, handler)?;
        self.hooks.push(hook);

        Some(())
    }

    pub fn enable(&mut self) -> Option<()> {
        // TODO: Should we update an internal state? Is it a problem if we do it multiple times?

        // Split 2mb page into 4kb pages, and set the hooked page to RW
        //
        for hook in self.hooks.iter() {
            let large_page_base = hook.original_pa.align_down_to_large_page().as_u64();
            let base_page_base = hook.original_pa.align_down_to_base_page().as_u64();

            self.npt.split_2mb_to_4kb(large_page_base)?;
            self.npt
                .change_page_permission(base_page_base, base_page_base, AccessType::ReadWrite);

            // Install the hook here
            //
            // unsafe { hook.page.as_ptr().offset(4).write_volatile(0x42) };
            unsafe { HOOK.as_ref().unwrap().enable() };
        }

        Some(())
    }

    pub fn disable(&mut self) {
        // TODO: Implement this
    }

    /// Tries to find a hook for the specified faulting physical address.
    pub fn find_hook(&self, faulting_pa: u64) -> Option<&Hook> {
        // TODO: Assumes that both addresses are 4kb pages.

        let faulting_pa = PhysicalAddress::from_pa(faulting_pa);
        let faulting_pa = faulting_pa.align_down_to_base_page();

        for hook in self.hooks.iter() {
            let hook_pa = hook.original_pa.align_down_to_base_page();

            if hook_pa == faulting_pa {
                return Some(hook);
            }
        }

        None
    }

    /// Hides all the hooks by resetting the pages to their original state. This also resets the page
    /// permission to RW to wait for the next caller.
    pub fn hide_hooks(&mut self) -> Option<()> {
        for hook in self.hooks.iter() {
            let guest_pa = hook.original_pa.align_down_to_base_page().as_u64();
            self.npt
                .change_page_permission(guest_pa, guest_pa, AccessType::ReadWrite);
        }

        Some(())
    }
}
