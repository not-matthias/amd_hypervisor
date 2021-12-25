use crate::nt::include::{RtlClearAllBits, RtlInitializeBitMap, RtlSetBits, RTL_BITMAP};
use crate::nt::memory::{alloc_contiguous, PAGE_SIZE};
use nt::include::PVOID;

pub const IA32_MSR_PAT: usize = 0x00000277;
pub const IA32_MSR_EFER: usize = 0xc0000080;
pub const CHAR_BIT: usize = 8;
pub const BITS_PER_MSR: usize = 2;
pub const SECOND_MSR_RANGE_BASE: usize = 0xc0000000;
pub const SECOND_MSRPM_OFFSET: usize = 0x800 * CHAR_BIT;

pub const SVM_MSR_PERMISSIONS_MAP_SIZE: usize = PAGE_SIZE * 2;

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
        }

        Some(Self {
            bitmap: memory? as PVOID,
        })
    }

    pub fn build(self) -> Self {
        // Based on this: https://github.com/tandasat/SimpleSvm/blob/master/SimpleSvm/SimpleSvm.cpp#L1465
        //
        let mut bitmap_header: RTL_BITMAP = unsafe { core::mem::zeroed() };
        let bitmap_header_ptr = &mut bitmap_header as *mut RTL_BITMAP;

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
        let offset = (IA32_MSR_EFER - SECOND_MSR_RANGE_BASE) * BITS_PER_MSR;
        let offset = SECOND_MSRPM_OFFSET + offset;

        // TODO: Figure out what this exactly does.

        // Set the MSB bit indicating write accesses to the MSR should be
        // intercepted.
        //
        unsafe { RtlSetBits(bitmap_header_ptr as _, (offset + 1) as u32, 1) };

        self
    }
}
