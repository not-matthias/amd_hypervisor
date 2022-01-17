extern crate alloc;

use crate::{
    hook::npt::DuplicateNptHook,
    svm::{
        data::msr_bitmap::MsrBitmap,
        msr::{SVM_MSR_TSC, SVM_MSR_VM_HSAVE_PA},
    },
    utils::memory::AllocatedMemory,
    Hook,
};
use alloc::vec::Vec;
use x86::msr::IA32_EFER;

#[repr(C)]
pub struct SharedData {
    pub msr_bitmap: AllocatedMemory<MsrBitmap>,
    pub hooked_npt: AllocatedMemory<DuplicateNptHook>,
}

impl SharedData {
    pub fn new(hooks: Vec<Hook>) -> Option<AllocatedMemory<Self>> {
        log::info!("Creating shared data");

        let mut data = AllocatedMemory::<Self>::alloc(core::mem::size_of::<Self>())?;
        data.msr_bitmap = {
            let mut bitmap = MsrBitmap::new()?;

            bitmap.hook_msr(IA32_EFER);
            bitmap.hook_rdmsr(SVM_MSR_TSC);
            bitmap.hook_rdmsr(SVM_MSR_VM_HSAVE_PA);

            bitmap
        };

        data.hooked_npt = DuplicateNptHook::new(hooks)?;

        Some(data)
    }
}
