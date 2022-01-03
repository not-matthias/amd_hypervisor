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
    address: u64,

    /// The physical address of the original function.
    physical_address: PhysicalAddress,
    handler: *const (),

    page: AllocatedMemory<u8>,
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

        Some(Self {
            address,
            physical_address: PhysicalAddress::from_va(address),
            handler,
            page: Self::copy_page(address)?,
        })
    }
}

pub struct HookedNpt {
    pub npt: AllocatedMemory<NestedPageTable>,

    // TODO: Can we remove these useless allocations?
    hooks: Vec<Hook>,
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
            self.npt
                .split_2mb_to_4kb(hook.physical_address.aligned_pa())?;
            self.npt
                .change_page_permission(hook.physical_address.aligned_pa(), AccessType::READ_WRITE);
        }

        Some(())
    }

    pub fn disable(&mut self) {
        // TODO: Implement this
    }

    pub fn visible() -> bool {
        // TODO: Check if the first byte at the address is 0xcc

        false
    }
}
