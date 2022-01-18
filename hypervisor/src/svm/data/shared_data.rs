extern crate alloc;

use crate::{
    hook::{npt::DuplicateNptHook, Hook},
    svm::{
        data::msr_bitmap::MsrBitmap,
        msr::{SVM_MSR_TSC, SVM_MSR_VM_HSAVE_PA},
    },
    utils::alloc::PhysicalAllocator,
};
use alloc::{boxed::Box, vec::Vec};
use x86::msr::IA32_EFER;

#[repr(C)]
pub struct SharedData {
    pub msr_bitmap: Box<MsrBitmap, PhysicalAllocator>,
    pub hooked_npt: Box<DuplicateNptHook>,
}

impl SharedData {
    pub fn new(hooks: Vec<Hook>) -> Option<Box<Self>> {
        log::info!("Creating shared data");

        Some(Box::new(Self {
            msr_bitmap: {
                let mut bitmap = MsrBitmap::new();

                bitmap.hook_msr(IA32_EFER);
                bitmap.hook_rdmsr(SVM_MSR_TSC);
                bitmap.hook_rdmsr(SVM_MSR_VM_HSAVE_PA);

                bitmap
            },
            hooked_npt: DuplicateNptHook::new(hooks)?,
        }))
    }
}