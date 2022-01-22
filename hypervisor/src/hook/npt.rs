use crate::{
    hook::{Hook, HookType},
    svm::{data::nested_page_table::NestedPageTable, paging::AccessType},
    utils::addresses::PhysicalAddress,
};
use alloc::{boxed::Box, vec::Vec};

pub struct DuplicateNptHook {
    pub primary_npt: Box<NestedPageTable>,
    pub primary_pml4: PhysicalAddress,

    /// This is the nested page table, where the hooked pages are set to RWX and
    /// the original pages are set to RW. Because of this, we can detect
    /// when the hooked page has been left.
    pub secondary_npt: Box<NestedPageTable>,
    pub secondary_pml4: PhysicalAddress,

    pub hooks: Vec<Hook>,
}

// TODO: Remove the hooking stuff and either move it to a different crate or
// remove it entirely.
// TODO: Can we somehow let the user specify which page table to use? System,
//       Duplicate, Normal, None?

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

            self.primary_npt
                .change_page_permission(page, page, AccessType::ReadWrite);
            self.secondary_npt.change_page_permission(
                page,
                hook_page,
                AccessType::ReadWriteExecute,
            );
        }

        Some(())
    }

    pub fn new(hooks: Vec<Hook>) -> Option<Box<Self>> {
        let primary_npt = NestedPageTable::identity_4kb(AccessType::ReadWriteExecute);
        let primary_pml4 = PhysicalAddress::from_va(primary_npt.pml4.as_ptr() as u64);

        let secondary_npt = NestedPageTable::identity_4kb(AccessType::ReadWrite);
        let secondary_pml4 = PhysicalAddress::from_va(secondary_npt.pml4.as_ptr() as u64);

        let mut instance = Self {
            primary_npt,
            primary_pml4,
            //
            secondary_npt,
            secondary_pml4,
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
