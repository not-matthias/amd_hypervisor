use crate::{
    svm::{msr_bitmap::MsrBitmap, nested_page_table::NestedPageTable},
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
    #[cfg(feature = "secondary-npt")]
    pub fn new(
        primary_npt: Box<NestedPageTable>, secondary_npt: Box<NestedPageTable>,
    ) -> Option<Box<Self>> {
        let primary_pml4 = PhysicalAddress::from_va(primary_npt.pml4.as_ptr() as u64);
        let secondary_pml4 = PhysicalAddress::from_va(secondary_npt.pml4.as_ptr() as u64);

        Some(Box::new(Self {
            msr_bitmap: {
                let mut bitmap = MsrBitmap::new();
                bitmap.hook_msr(IA32_EFER);
                bitmap
            },

            primary_npt,
            primary_pml4,

            secondary_npt,
            secondary_pml4,
        }))
    }

    #[cfg(not(feature = "secondary-npt"))]
    pub fn new(primary_npt: Box<NestedPageTable>) -> Option<Box<Self>> {
        let primary_pml4 = PhysicalAddress::from_va(primary_npt.pml4.as_ptr() as u64);

        Some(Box::new(Self {
            msr_bitmap: {
                let mut bitmap = MsrBitmap::new();
                bitmap.hook_msr(IA32_EFER);
                bitmap
            },

            primary_npt,
            primary_pml4,
        }))
    }
}
