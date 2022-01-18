extern crate alloc;

use crate::utils::{
    addresses::PhysicalAddress,
    function_hook::FunctionHook,
    nt::{irql::assert_paged_code, MmGetSystemRoutineAddress, RtlCopyMemory},
};
use alloc::boxed::Box;
use windy::{UnicodeString, WStr};
use x86::bits64::paging::{PAddr, VAddr, BASE_PAGE_SIZE};
use x86_64::instructions::interrupts::without_interrupts;

pub mod handlers;
pub mod npt;
pub mod testing;

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
        let wide_string = unsafe { WStr::from_raw(wide_string.as_ptr()) }; // TODO: Check if this contains the null character

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
