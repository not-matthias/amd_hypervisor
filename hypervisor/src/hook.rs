extern crate alloc;

use crate::{
    svm::{data::nested_page_table::NestedPageTable, paging::AccessType},
    utils::{
        addresses::PhysicalAddress,
        function_hook::FunctionHook,
        nt::{irql::assert_paged_code, MmGetSystemRoutineAddress, RtlCopyMemory},
    },
};
use alloc::{boxed::Box, vec::Vec};
use windy::{UnicodeString, WStr};
use x86::bits64::paging::{PAddr, VAddr, BASE_PAGE_SIZE};
use x86_64::instructions::interrupts::without_interrupts;

pub enum HookType {
    /// Creates a shadow page to hook a function.
    Function { inline_hook: FunctionHook },

    /// Creates a shadow page to hide some data.
    Page,
}

pub struct Hook {
    // Addresses of the original function / page.
    pub original_va: u64,
    pub original_pa: PhysicalAddress,

    // Addresses of the copied page. If it's a function, the exact location inside the page is
    // stored.
    pub hook_va: u64,
    pub hook_pa: PhysicalAddress,

    pub page: Box<[u8]>,
    pub page_va: u64,
    pub page_pa: PhysicalAddress,

    pub hook_type: HookType,
}

impl Hook {
    /// Creates a copy of the specified page.
    ///
    /// ## Why does this code have to be paged?
    ///
    /// Because otherwise the code could be paged out, which will result in a
    /// page fault. We must make sure, that this code must be called at IRQL
    /// < DISPATCH_LEVEL.
    ///
    /// For more information, see the official docs:https://docs.microsoft.com/en-us/windows-hardware/drivers/kernel/when-should-code-and-data-be-pageable-
    fn copy_page(address: u64) -> Option<Box<[u8]>> {
        log::info!("Creating a copy of the page");

        let page_address = PAddr::from(address).align_down_to_base_page();
        if page_address.is_zero() {
            log::error!("Invalid address: {:#x}", address);
            return None;
        }
        let mut page = Box::new_uninit_slice(BASE_PAGE_SIZE);

        log::info!("Page address: {:#x}", page_address);

        // TODO: Figure out why this doesn't compile (KeGetCurrentIrql not found)
        #[cfg(debug_assertions)]
        assert_paged_code!();

        without_interrupts(|| {
            unsafe {
                RtlCopyMemory(
                    page.as_mut_ptr() as _,
                    page_address.as_u64() as *mut u64,
                    BASE_PAGE_SIZE,
                )
            };
        });

        Some(unsafe { page.assume_init() })
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

    pub fn hook_function_ptr(function_ptr: u64, handler: *const ()) -> Option<Self> {
        let original_pa = PhysicalAddress::from_va(function_ptr);
        log::info!("Physical address: {:#x}", original_pa.as_u64());

        let page = Self::copy_page(function_ptr)?;
        let page_va = page.as_ptr() as *mut u64 as u64;
        let page_pa = PhysicalAddress::from_va(page_va);

        let hook_va = Self::address_in_page(page_va, function_ptr);
        let hook_pa = PhysicalAddress::from_va(hook_va);

        // Install inline hook on the **copied** page (not the original one).
        //
        let inline_hook = FunctionHook::new(function_ptr, hook_va, handler)?;

        Some(Self {
            original_va: function_ptr,
            original_pa,
            hook_va,
            hook_pa,
            page,
            page_va,
            page_pa,
            hook_type: HookType::Function { inline_hook },
        })
    }

    pub fn hook_function(name: &str, handler: *const ()) -> Option<Self> {
        let wide_string = widestring::U16CString::from_str(name).ok()?;
        let wide_string = unsafe { WStr::from_raw(wide_string.as_ptr()) };

        let mut wide_string = UnicodeString::new(wide_string);
        let address = unsafe { MmGetSystemRoutineAddress(wide_string.as_mut_ptr() as _) };
        if address.is_null() {
            log::error!("Could not find function: {}", name);
            return None;
        }

        log::info!("Found function address of {}: {:p}", name, address);

        Self::hook_function_ptr(address as u64, handler)
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

/// Helper structure to manage hooks.
pub struct HookManager {
    pub hooks: Vec<Hook>,
}

impl HookManager {
    pub fn new(hooks: Vec<Hook>) -> Self {
        Self { hooks }
    }

    /// Enables the hook by setting the permission of the primary page to read
    /// write. Now when the guest tries to execute this page, a page fault will
    /// occur and we can switch to the secondary npt (needs to be implemented).
    ///
    /// The secondary npt has only the hooked page set to RWX, so we'll get
    /// another page fault once the hooked page has been left. We can then
    /// restore the execution back to the primary npt.
    ///
    /// For more information, see: [AMD-V for Hackers](https://tandasat.github.io/VXCON/AMD-V_for_Hackers.pdf)
    pub fn enable_hooks(
        &self, primary_npt: &mut NestedPageTable, secondary_npt: &mut NestedPageTable,
    ) {
        for hook in &self.hooks {
            if let HookType::Function { inline_hook } = &hook.hook_type {
                inline_hook.enable()
            }

            let page = hook.original_pa.align_down_to_base_page().as_u64();
            let hook_page = hook.hook_pa.align_down_to_base_page().as_u64();

            primary_npt.change_page_permission(page, page, AccessType::ReadWrite);
            secondary_npt.change_page_permission(page, hook_page, AccessType::ReadWriteExecute);
        }
    }

    /// Tries to find a hook for the specified faulting physical address.
    ///
    /// ## Assumptions
    ///
    /// Both pages have to be 4kb pages, because the comparison is done by
    /// comparing the base page aligned physical addresses. This will most
    /// likely not be a problem, because we only use 4kb pages for hooks
    /// anyways.
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
}
