#[allow(unused)]
#[allow(dead_code)]
extern crate alloc;

use alloc::vec::Vec;
use nt::kernel::get_system_routine_address;
use x86::current::paging::PML4;

pub mod physmem_descriptor;

pub enum NptState {
    Default,
    HookEnabledInvisible,
    HookEnabledVisible,
}

// TODO: Figure out what those fields do
pub struct HookEntry {
    pub hook_address: u64,
    pub hook_handler: *const (),

    page_base_for_execution: *mut u64,
    phy_page_base: *mut u64,
    phy_page_base_for_execution: *mut u64,

    original_call_stub: *mut u64,
}

impl HookEntry {
    pub fn new(_address: u64, _handler: *const ()) -> Self {
        // let mut hook_entry = Self::default();
        // hook_entry.hook_address = address;
        // hook_entry.hook_handler = handler;

        todo!()
    }
}

pub struct HookData {
    pml4: *mut PML4,

    // Preallocated npt entries
    // TODO: ???
    //
    preallocated_npt_entries: [u64; 50],
    used_preallocated_npt_entries: u32,

    // TODO: ???
    //
    max_npt_pdp_entries_used: u32,

    active_hook_entry: *mut HookEntry,

    npt_state: NptState,
}

// This is actually stored inside the vcpu struct
impl HookData {
    pub fn new() -> Self {
        todo!()
    }

    pub fn build_npt() {
        // TODO:
        // - Allocate pml4
        // - Build npt based on physical memory ranges
        // - Build sub tables (for APIC base?)
        // - COmpute max pdpt index
    }

    pub fn init_preallocated_npt_entries(&mut self) {
        // TODO:
        // - Allocate npt entries
    }
}

// ============================================================================

pub struct Hook {
    physmem_descriptor: PhysmemDescriptor,
}

impl Hook {
    pub fn new() -> Option<Self> {
        // TODO: InitializeHookRegistrationEntries

        Some(Self {
            physmem_descriptor: PhysmemDescriptor::new()?,
        })
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

    pub fn create_exec_page(_hook_address: usize) {
        // GetSharedMemoryEntry, can be static
        //
        //

        // ExAllocatePoolWithTag
        // Assert page aligned
        // IoAllocateMdl
        // MmProbeAndLockPages

        // SharedMemoryEntry { hook_address_base, exec_page, hook_address_mdl }
    }

    pub fn install_hook(_exec_page: (), _hook_address: usize) {
        // InstallHookOnExecPage, installs hook (0xcc)

        // let original_call_stub: () = ();

        // Find first instruction (requires a disassembler) - FindFirstInstruction()
        // return ERROR if it's across 2 pages

        // Allocate executable memory that contains the stub
        // Initialize stub

        // Install breakpoint on page to know when the page is executed

        // Invalidate caches

        // TODO: Why do we store the call stub and not the entire page?
    }

    pub fn visible() -> bool {
        // TODO: Check if the first byte at the address is 0xcc

        false
    }
}
