extern crate alloc;

use crate::nt::addresses::PhysicalAddress;
use crate::nt::include::{assert_paged_code, RtlCopyMemory};
use crate::nt::inline_hook::InlineHook;
use crate::nt::memory::AllocatedMemory;
use crate::svm::data::nested_page_table::NestedPageTable;
use crate::svm::paging::AccessType;

use alloc::vec::Vec;

use nt::kernel::get_system_routine_address;

use x86::bits64::paging::{PAddr, VAddr, BASE_PAGE_SIZE};

use x86_64::instructions::interrupts::without_interrupts;

pub mod handlers;
pub mod testing;

pub enum HookType {
    /// Creates a shadow page to hook a function.
    Function {
        inline_hook: AllocatedMemory<InlineHook>,
    },

    /// Creates a shadow page to hide some data.
    Page,
}

pub struct Hook {
    // Addresses of the original function / page.
    //
    pub original_va: u64,
    pub original_pa: PhysicalAddress,

    // Addresses of the copied page. If it's a function, the exact location inside the page is stored.
    //
    pub hook_va: u64,
    pub hook_pa: PhysicalAddress,

    pub page: AllocatedMemory<u8>,
    pub page_va: u64,
    pub page_pa: PhysicalAddress,

    pub hook_type: HookType,
}

impl Hook {
    /// Creates a copy of the specified page.
    ///
    /// ## Why does this code have to be paged?
    ///
    /// Because otherwise the code could be paged out, which will result in a page fault. We must
    /// make sure, that this code must be called at IRQL < DISPATCH_LEVEL.
    ///
    /// For more information, see the official docs:https://docs.microsoft.com/en-us/windows-hardware/drivers/kernel/when-should-code-and-data-be-pageable-
    fn copy_page(address: u64) -> Option<AllocatedMemory<u8>> {
        log::info!("Creating a copy of the page");

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

        Some(page)
    }

    /// Returns the address of the specified function in the copied page.
    ///
    /// ## Parameters
    /// - `page_start`: The start address of the page (virtual address).
    /// - `address`: The address of the function outside the copied page.
    ///
    /// ## Returns
    ///
    /// Returns the address of the function inside the copied page.
    fn address_in_page(page_start: u64, address: u64) -> u64 {
        let base_offset = VAddr::from(address).base_page_offset();

        page_start + base_offset
    }

    pub fn hook_function(name: &str, handler: *const ()) -> Option<Self> {
        let address = get_system_routine_address(name)? as u64;
        log::info!("Found function address of {}: {:#x}", &name, address);

        let original_pa = PhysicalAddress::from_va(address);
        log::info!("Physical address: {:#x}", original_pa.as_u64());

        // TODO: Remove some of these useless variables.

        let page = Self::copy_page(address)?;
        let page_va = page.as_ptr() as *mut u64 as u64;
        let page_pa = PhysicalAddress::from_va(page_va);

        let hook_va = Self::address_in_page(page_va, address);
        let hook_pa = PhysicalAddress::from_va(hook_va);

        // Install inline hook on the **copied** page (not the original one).
        //
        let inline_hook = InlineHook::new(address, hook_va, handler)?;

        Some(Self {
            original_va: address,
            original_pa,
            hook_va,
            hook_pa,
            page,
            page_va,
            page_pa,
            hook_type: HookType::Function { inline_hook },
        })
    }

    pub fn hook_page(address: u64) -> Option<Self> {
        let original_pa = PhysicalAddress::from_va(address);

        let page = Self::copy_page(address)?;
        let page_va = page.as_ptr() as *mut u64 as u64;
        let page_pa = PhysicalAddress::from_va(page_va);

        let hook_va = page_va;
        let hook_pa = PhysicalAddress::from_va(hook_va);

        Some(Self {
            original_va: address,
            original_pa,
            page_va,
            page_pa,
            hook_va,
            hook_pa,
            page,
            hook_type: HookType::Page,
        })
    }
}

pub struct HookedNpt {
    pub npt: AllocatedMemory<NestedPageTable>,
    pub hooks: Vec<Hook>,
}

impl HookedNpt {
    pub fn new(hooks: Vec<Hook>) -> Option<AllocatedMemory<Self>> {
        let mut hooked_npt = AllocatedMemory::<Self>::alloc(core::mem::size_of::<Self>())?;

        hooked_npt.npt = NestedPageTable::identity_2mb()?;
        hooked_npt.hooks = hooks;

        Some(hooked_npt)
    }

    pub fn enable(&mut self) -> Option<()> {
        // TODO: Should we update an internal state? Is it a problem if we do it multiple times?

        // Split 2mb page into 4kb pages, and set the hooked page to RW
        //
        for hook in self.hooks.iter() {
            let large_page_base = hook.original_pa.align_down_to_large_page().as_u64();
            let base_page_base = hook.original_pa.align_down_to_base_page().as_u64();

            // Enable inline hook
            //
            if let HookType::Function { inline_hook } = &hook.hook_type {
                inline_hook.enable()
            }

            self.npt.split_2mb_to_4kb(large_page_base)?;
            self.npt
                .change_page_permission(base_page_base, base_page_base, AccessType::ReadWrite);
        }

        Some(())
    }

    pub fn disable(&mut self) {
        todo!()
    }

    /// Tries to find a hook for the specified faulting physical address.
    ///
    /// ## Assumptions
    ///
    /// Both pages have to be 4kb pages, because the comparison is done by comparing the base page
    /// aligned physical addresses. This will most likely not be a problem, because we only use
    /// 4kb pages for hooks anyways.
    ///
    pub fn find_hook(&self, faulting_pa: u64) -> Option<&Hook> {
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

    /// Tries to find a hook for the specified hook virtual address.
    pub fn find_hook_by_address(&self, address: u64) -> Option<&Hook> {
        for hook in self.hooks.iter() {
            if hook.original_va == address {
                return Some(hook);
            }
        }

        None
    }

    /// Hides all the hooks by resetting the pages to their original state.
    pub fn hide_hooks(&mut self) {
        for hook in self.hooks.iter() {
            let guest_pa = hook.original_pa.align_down_to_base_page().as_u64();
            self.npt
                .change_page_permission(guest_pa, guest_pa, AccessType::ReadWrite);
        }
    }
}
