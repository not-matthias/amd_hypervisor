#[allow(unused)]
#[allow(dead_code)]
extern crate alloc;

use alloc::vec::Vec;
use nt::kernel::get_system_routine_address;
use x86::current::paging::PML4;

pub struct HookedNpt {}

impl HookedNpt {
    pub fn new() -> Option<Self> {
        Some(Self {})
    }

    /// Install the hooks on the passed functions.
    pub fn install(functions: &[&str]) {
        let _hook_entries = Vec::<u8>::new();

        for &function in functions {
            let _address = get_system_routine_address(function).unwrap();

            // let hook_entry = HookEntry::new(address as u64, handler);

            // Get a memory resource for install hook on the address.
            //
        }
    }

    pub fn visible() -> bool {
        // TODO: Check if the first byte at the address is 0xcc

        false
    }
}
