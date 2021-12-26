use crate::nt::include::{RtlClearAllBits, RtlInitializeBitMap, RtlSetBits, RTL_BITMAP};
use crate::nt::memory::{alloc_contiguous, PAGE_SIZE};
use core::mem::MaybeUninit;
use nt::include::PVOID;
use x86::msr::IA32_EFER;

pub const SVM_MSR_VM_HSAVE_PA: u32 = 0xc0010117;
pub const EFER_SVME: u64 = 1 << 12;
pub const CHAR_BIT: u32 = 8;
pub const BITS_PER_MSR: u32 = 2;
pub const SECOND_MSR_RANGE_BASE: u32 = 0xc0000000;
pub const SECOND_MSRPM_OFFSET: u32 = 0x800 * CHAR_BIT;

pub const SVM_MSR_PERMISSIONS_MAP_SIZE: u32 = (PAGE_SIZE * 2) as u32;

pub struct MsrBitmap {
    pub bitmap: PVOID,
}

impl MsrBitmap {
    pub fn new() -> Option<Self> {
        // The MSR permissions bitmap consists of four separate bit vectors of 16
        // Kbits (2 Kbytes) each. See: `15.11 - MSR Intercepts`.
        //
        let memory = alloc_contiguous(PAGE_SIZE * 2);
        if memory.is_none() {
            log::warn!("Failed to allocate memory for MSR permission map");
            return None;
        }
        log::trace!("Allocated memory for MSR permission map: {:x?}", memory);

        Some(Self {
            bitmap: memory? as PVOID,
        })
    }

    pub fn build(self) -> Self {
        log::info!("Building msr permission bitmap");

        // Based on this: https://github.com/tandasat/SimpleSvm/blob/master/SimpleSvm/SimpleSvm.cpp#L1465
        //
        let mut bitmap_header: MaybeUninit<RTL_BITMAP> = MaybeUninit::uninit();
        let bitmap_header_ptr = bitmap_header.as_mut_ptr() as *mut RTL_BITMAP;

        // Setup and clear all bits, indicating no MSR access should be intercepted.
        //
        unsafe {
            RtlInitializeBitMap(
                bitmap_header_ptr as _,
                self.bitmap as _,
                (SVM_MSR_PERMISSIONS_MAP_SIZE * CHAR_BIT) as u32,
            )
        }
        unsafe { RtlClearAllBits(bitmap_header_ptr as _) }

        // Compute an offset from the second MSR permissions map offset (0x800) for
        // IA32_MSR_EFER in bits. Then, add an offset until the second MSR
        // permissions map.
        //
        let offset = (IA32_EFER - SECOND_MSR_RANGE_BASE) * BITS_PER_MSR;
        let offset = SECOND_MSRPM_OFFSET + offset;

        // TODO: Figure out what this exactly does.

        // Set the MSB bit indicating write accesses to the MSR should be
        // intercepted.
        //
        unsafe { RtlSetBits(bitmap_header_ptr as _, (offset + 1) as u32, 1) };

        self
    }
}
