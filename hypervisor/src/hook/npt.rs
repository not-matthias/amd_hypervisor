use crate::{
    hook::{Hook, HookType},
    svm::{data::nested_page_table::NestedPageTable, paging::AccessType},
    utils::addresses::PhysicalAddress,
};
use alloc::{boxed::Box, vec::Vec};

pub struct DuplicateNptHook {
    pub rwx_npt: Box<NestedPageTable>,
    pub rwx_pml4: PhysicalAddress,

    /// This is the nested page table, where the hooked pages are set to RWX and
    /// the original pages are set to RW. Because of this, we can detect
    /// when the hooked page has been left.
    pub rw_npt: Box<NestedPageTable>,
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

    pub fn new(hooks: Vec<Hook>) -> Option<Box<Self>> {
        let rwx_npt = NestedPageTable::identity_4kb(AccessType::ReadWriteExecute);
        let rwx_pml4 = PhysicalAddress::from_va(rwx_npt.pml4.as_ptr() as u64);

        let rw_npt = NestedPageTable::identity_4kb(AccessType::ReadWrite);
        let rw_pml4 = PhysicalAddress::from_va(rw_npt.pml4.as_ptr() as u64);

        let mut instance = Self {
            rwx_npt,
            rwx_pml4,
            //
            rw_npt,
            rw_pml4,
            //
            hooks,
        };
        instance.enable_hooks()?;

        Some(Box::new(instance))
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
