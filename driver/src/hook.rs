#![allow(dead_code)]
#![allow(unused)]

extern crate alloc;

use crate::dbg_break;
use crate::nt::addresses::PhysicalAddress;
use crate::nt::include::{assert_paged_code, RtlCopyMemory};
use crate::nt::memory::AllocatedMemory;
use crate::svm::data::nested_page_table::NestedPageTable;
use crate::svm::paging::AccessType;
use alloc::string::String;
use alloc::vec::Vec;
use nt::kernel::get_system_routine_address;
use x86::bits64::paging::{PAddr, BASE_PAGE_SIZE};
use x86_64::instructions::interrupts::without_interrupts;

pub struct Hook {
    /// The address of the original function.    
    pub address: u64,

    /// The physical address of the original function.
    pub physical_address: PhysicalAddress,

    // TODO: Unused for now
    pub handler: *const (),

    pub page: AllocatedMemory<u8>,
}

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

    pub fn new(name: &str, handler: *const ()) -> Option<Self> {
        let address = get_system_routine_address(name)? as u64;
        log::info!("Found address of {}: {:#x}", &name, address);

        let physical_address = PhysicalAddress::from_va(address);
        log::info!("Physical address: {:#x}", physical_address.as_u64());

        Some(Self {
            address,
            physical_address,
            handler,
            page: Self::copy_page(address)?,
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
            let large_page_base = hook.physical_address.align_down_to_large_page().as_u64();
            let base_page_base = hook.physical_address.align_down_to_base_page().as_u64();

            self.npt.split_2mb_to_4kb(large_page_base)?;
            self.npt
                .change_page_permission(base_page_base, AccessType::ReadWrite);
        }

        Some(())
    }

    pub fn disable(&mut self) {
        // TODO: Implement this
    }

    pub fn exists_hook(&self, faulting_pa: u64) -> bool {
        // TODO: Assumes that both addresses are 4kb pages.

        let faulting_pa = PhysicalAddress::from_pa(faulting_pa);
        let faulting_pa = faulting_pa.align_down_to_base_page();

        for hook in self.hooks.iter() {
            let hook_pa = hook.physical_address.align_down_to_base_page();

            if hook_pa == faulting_pa {
                return true;
            }
        }

        false
    }
}
