extern crate alloc;

use crate::{
    svm::{
        data::{msr_bitmap::MsrBitmap, nested_page_table::NestedPageTable},
        paging::AccessType,
    },
    utils::{addresses::PhysicalAddress, alloc::PhysicalAllocator},
};
use alloc::boxed::Box;
use x86::msr::IA32_EFER;

#[repr(C)]
pub struct SharedData {
    pub msr_bitmap: Box<MsrBitmap, PhysicalAllocator>,

    pub primary_npt: Box<NestedPageTable>,
    pub primary_pml4: PhysicalAddress,

    #[cfg(feature = "secondary-npt")]
    pub secondary_npt: Box<NestedPageTable>,
    #[cfg(feature = "secondary-npt")]
    pub secondary_pml4: PhysicalAddress,
}

impl SharedData {
    pub fn new() -> Option<Box<Self>> {
        log::info!("Creating shared data");

        // TODO: How to allow the user to set their own protections for hooks etc?

        let primary_npt = NestedPageTable::identity_4kb(AccessType::ReadWriteExecute);
        let primary_pml4 = PhysicalAddress::from_va(primary_npt.pml4.as_ptr() as u64);

        #[cfg(feature = "secondary-npt")]
        let secondary_npt = NestedPageTable::identity_4kb(AccessType::ReadWrite);
        #[cfg(feature = "secondary-npt")]
        let secondary_pml4 = PhysicalAddress::from_va(primary_npt.pml4.as_ptr() as u64);

        Some(Box::new(Self {
            msr_bitmap: {
                let mut bitmap = MsrBitmap::new();
                bitmap.hook_msr(IA32_EFER);
                bitmap
            },

            primary_npt,
            primary_pml4,

            #[cfg(feature = "secondary-npt")]
            secondary_npt,

            #[cfg(feature = "secondary-npt")]
            secondary_pml4,
        }))
    }
}
