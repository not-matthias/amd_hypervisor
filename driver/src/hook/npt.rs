use crate::nt::addresses::PhysicalAddress;
use crate::nt::memory::AllocatedMemory;
use crate::svm::data::nested_page_table::NestedPageTable;
use crate::svm::paging::AccessType;
use crate::{Hook, HookType};
use alloc::vec::Vec;

pub struct DuplicateNptHook {
    pub rwx_npt: AllocatedMemory<NestedPageTable>,
    pub rwx_pml4: PhysicalAddress,

    /// This is the nested page table, where the hooked pages are set to RWX and the original pages
    /// are set to RW. Because of this, we can detect when the hooked page has been left.
    pub rw_npt: AllocatedMemory<NestedPageTable>,
    pub rw_pml4: PhysicalAddress,

    pub hooks: Vec<Hook>,
}

impl DuplicateNptHook {
    fn enable_hooks(&mut self) -> Option<()> {
        for hook in &self.hooks {
            // Enable inline hook
            //
            if let HookType::Function { inline_hook } = &hook.hook_type {
                inline_hook.enable()
            }

            let page = hook.original_pa.align_down_to_base_page().as_u64();
            let hook_page = hook.hook_pa.align_down_to_base_page().as_u64();

            self.rwx_npt
                .change_page_permission(page, page, AccessType::ReadWrite);
            self.rw_npt
                .change_page_permission(page, hook_page, AccessType::ReadWriteExecute);
        }

        Some(())
    }

    pub fn new(hooks: Vec<Hook>) -> Option<AllocatedMemory<Self>> {
        let mut hooked_npt = AllocatedMemory::<Self>::alloc(core::mem::size_of::<Self>())?;

        hooked_npt.rwx_npt = NestedPageTable::identity_4kb(AccessType::ReadWriteExecute)?;
        hooked_npt.rwx_pml4 = PhysicalAddress::from_va(hooked_npt.rwx_npt.pml4.as_ptr() as u64);

        hooked_npt.rw_npt = NestedPageTable::identity_4kb(AccessType::ReadWrite)?;
        hooked_npt.rw_pml4 = PhysicalAddress::from_va(hooked_npt.rw_npt.pml4.as_ptr() as u64);

        hooked_npt.hooks = hooks;

        // Enable the hooks
        //
        hooked_npt.enable_hooks()?;

        Some(hooked_npt)
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
}

pub struct RetAddrHook {
    pub npt: AllocatedMemory<NestedPageTable>,
    pub hooks: Vec<Hook>,
}

impl RetAddrHook {
    pub fn new(hooks: Vec<Hook>) -> Option<AllocatedMemory<Self>> {
        let mut hooked_npt = AllocatedMemory::<Self>::alloc(core::mem::size_of::<Self>())?;

        hooked_npt.npt = NestedPageTable::identity_2mb(AccessType::ReadWriteExecute)?;
        hooked_npt.hooks = hooks;

        Some(hooked_npt)
    }

    pub fn enable(&mut self) -> Option<()> {
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

            self.npt
                .split_2mb_to_4kb(large_page_base, AccessType::ReadWriteExecute);
            self.npt
                .change_page_permission(base_page_base, base_page_base, AccessType::ReadWrite);
        }

        Some(())
    }

    // TODO: Can we somehow make this generic?
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

    // TODO: Can we somehow make this generic?
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
