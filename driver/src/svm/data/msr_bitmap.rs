use crate::nt::include::{RtlClearAllBits, RtlInitializeBitMap, RtlSetBits, RTL_BITMAP};
use crate::nt::memory::AllocatedMemory;

use core::mem::MaybeUninit;
use x86::bits64::paging::BASE_PAGE_SIZE;
use x86::msr::IA32_EFER;

pub const SVM_MSR_VM_HSAVE_PA: u32 = 0xc0010117;
pub const EFER_SVME: u64 = 1 << 12;
pub const CHAR_BIT: u32 = 8;
pub const BITS_PER_MSR: u32 = 2;
pub const SECOND_MSR_RANGE_BASE: u32 = 0xc0000000;
pub const SECOND_MSRPM_OFFSET: u32 = 0x800 * CHAR_BIT;

pub const SVM_MSR_PERMISSIONS_MAP_SIZE: u32 = (BASE_PAGE_SIZE * 2) as u32;

#[repr(C)]
pub struct Bitmap {
    /// 0000_0000 to 0000_1FFF
    pub msr_bitmap_0: [u8; 2048],
    /// C000_0000 to C000_1FFF
    pub msr_bitmap_1: [u8; 2048],
    /// C001_0000 to C001_1FFF
    pub msr_bitmap_2: [u8; 2048],
    /// Reserved
    pub msr_bitmap_3: [u8; 2048],
}
const_assert_eq!(core::mem::size_of::<Bitmap>(), 2 * BASE_PAGE_SIZE);
// TODO: Figure out how to use this instead.

pub struct MsrBitmap;

impl MsrBitmap {
    pub fn new() -> Option<AllocatedMemory<u32>> {
        let memory = AllocatedMemory::<u32>::alloc_contiguous(BASE_PAGE_SIZE * 2)?;

        // Setup the msr bitmap
        //
        Self::build(memory.as_ptr());

        Some(memory)
    }

    fn build(memory: *mut u32) {
        log::info!("Building msr permission bitmap");

        // Based on this: https://github.com/tandasat/SimpleSvm/blob/master/SimpleSvm/SimpleSvm.cpp#L1465
        //
        let mut bitmap_header: MaybeUninit<RTL_BITMAP> = MaybeUninit::uninit();
        let bitmap_header_ptr = bitmap_header.as_mut_ptr() as *mut _;

        // Setup and clear all bits, indicating no MSR access should be intercepted.
        //
        unsafe {
            RtlInitializeBitMap(
                bitmap_header_ptr as _,
                memory as _,
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

        // Set the MSB bit indicating write accesses to the MSR should be
        // intercepted.
        //
        unsafe { RtlSetBits(bitmap_header_ptr as _, (offset + 1) as u32, 1) };
    }
}
