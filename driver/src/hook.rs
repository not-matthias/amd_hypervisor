#![allow(dead_code)]
#![allow(unused)]

extern crate alloc;

use crate::nt::memory::AllocatedMemory;
use crate::svm::data::nested_page_table::NestedPageTable;
use alloc::string::String;
use alloc::vec::Vec;
use nt::kernel::get_system_routine_address;

pub struct Hook {
    address: usize,
    handler: *const (),

    page: AllocatedMemory<u8>,
}

impl Hook {
    pub fn new(name: String, handler: *const ()) -> Option<Self> {
        let address = get_system_routine_address(&name)?;
        log::info!("Found address of {}: {:#x}", &name, address);

        Some(Self {
            address,
            handler,
            page: AllocatedMemory::alloc_contiguous(0x1000)?,
        })
    }
}

pub struct HookedNpt {
    npt: AllocatedMemory<NestedPageTable>,
    hooks: Vec<Hook>,
}

impl HookedNpt {
    pub fn new() -> Option<Self> {
        Some(Self {
            npt: NestedPageTable::identity()?,
            hooks: Vec::new(),
        })
    }

    /// Hooks the specified function.
    ///
    /// ## Parameters
    ///
    /// - `function`: The name of the function to hook.
    /// - `handler`: The function that should be called from the hook.
    ///
    pub fn hook(mut self, function: String, handler: *const ()) -> Option<Self> {
        let hook = Hook::new(function, handler)?;
        self.hooks.push(hook);

        Some(self)
    }

    pub fn visible() -> bool {
        // TODO: Check if the first byte at the address is 0xcc

        false
    }
}
