#![allow(dead_code)]
#![allow(unused)]

extern crate alloc;

use crate::nt::memory::AllocatedMemory;
use crate::svm::data::nested_page_table::NestedPageTable;
use alloc::string::String;
use alloc::vec::Vec;
use nt::kernel::get_system_routine_address;
use x86::bits64::paging::PAddr;

pub struct Hook {
    address: usize,
    handler: *const (),

    page: AllocatedMemory<u8>,
}

impl Hook {
    fn copy_page(address: usize) -> AllocatedMemory<u8> {
        let page = PAddr::from(address).align_down_to_base_page();

        todo!()
    }

    pub fn new(name: &str, handler: *const ()) -> Option<Self> {
        let address = get_system_routine_address(name)?;
        log::info!("Found address of {}: {:#x}", &name, address);

        // TODO: Copy page

        Some(Self {
            address,
            handler,
            page: AllocatedMemory::alloc_contiguous(0x1000)?,
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

    pub fn visible() -> bool {
        // TODO: Check if the first byte at the address is 0xcc

        false
    }
}
