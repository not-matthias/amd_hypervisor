use crate::utils::{
    alloc::PhysicalAllocator,
    nt::{RtlClearAllBits, RtlInitializeBitMap, RtlSetBits, RTL_BITMAP},
};
use alloc::boxed::Box;
use core::mem::MaybeUninit;
use x86::bits64::paging::BASE_PAGE_SIZE;

const CHAR_BIT: u32 = 8;
const BITS_PER_MSR: u32 = 2;
const RANGE_SIZE: u32 = 0x800 * CHAR_BIT;

#[repr(C)]
pub struct MsrBitmap {
    /// 0000_0000 to 0000_1FFF
    pub msr_bitmap_0: [u8; 0x800],
    /// C000_0000 to C000_1FFF
    pub msr_bitmap_1: [u8; 0x800],
    /// C001_0000 to C001_1FFF
    pub msr_bitmap_2: [u8; 0x800],
    /// Reserved
    pub msr_bitmap_3: [u8; 0x800],
}
const_assert_eq!(core::mem::size_of::<MsrBitmap>(), 2 * BASE_PAGE_SIZE);

impl MsrBitmap {
    fn initialize_bitmap(bitmap_ptr: *mut u64) {
        let mut bitmap_header: MaybeUninit<RTL_BITMAP> = MaybeUninit::uninit();
        let bitmap_header_ptr = bitmap_header.as_mut_ptr() as *mut _;

        unsafe {
            RtlInitializeBitMap(
                bitmap_header_ptr as _,
                bitmap_ptr as _,
                core::mem::size_of::<Self>() as u32,
            )
        }
        unsafe { RtlClearAllBits(bitmap_header_ptr as _) }
    }

    fn bitmap_header(bitmap_ptr: *mut u64) -> RTL_BITMAP {
        // let mut bitmap_header: MaybeUninit<RTL_BITMAP> = MaybeUninit::uninit();
        // let bitmap_header_ptr = bitmap_header.as_mut_ptr() as *mut _;
        //
        // unsafe {
        //     RtlInitializeBitMap(
        //         bitmap_header_ptr as _,
        //         bitmap_ptr as _,
        //         core::mem::size_of::<Self>() as u32,
        //     )
        // }
        // unsafe { bitmap_header.assume_init() }

        RTL_BITMAP {
            SizeOfBitMap: core::mem::size_of::<Self>() as u32,
            Buffer: bitmap_ptr as _,
        }
    }

    pub fn new() -> Box<MsrBitmap, PhysicalAllocator> {
        let instance = Self {
            msr_bitmap_0: [0; 0x800],
            msr_bitmap_1: [0; 0x800],
            msr_bitmap_2: [0; 0x800],
            msr_bitmap_3: [0; 0x800],
        };
        let mut instance = Box::<Self, PhysicalAllocator>::new_in(instance, PhysicalAllocator);

        Self::initialize_bitmap(instance.as_mut() as *mut _ as _);

        instance
    }

    pub fn hook_msr(&mut self, msr: u32) {
        self.hook_wrmsr(msr);
        self.hook_rdmsr(msr);
    }

    pub fn hook_rdmsr(&mut self, msr: u32) {
        let offset = Self::msr_range(msr) + Self::msr_offset(msr);

        let mut bitmap_header = Self::bitmap_header(self as *mut _ as _);
        unsafe { RtlSetBits(&mut bitmap_header as *mut _, offset, 1) };
    }

    pub fn hook_wrmsr(&mut self, msr: u32) {
        let offset = Self::msr_range(msr) + Self::msr_offset(msr);

        let mut bitmap_header = Self::bitmap_header(self as *mut _ as _);
        unsafe { RtlSetBits(&mut bitmap_header as *mut _, offset + 1, 1) };
    }

    fn msr_offset(msr: u32) -> u32 {
        (msr & 0xfff) * BITS_PER_MSR
    }

    /// Returns the offset to the range for the specified MSR.
    #[allow(clippy::identity_op)]
    fn msr_range(msr: u32) -> u32 {
        if (0x0000_0000..=0x0000_1FFF).contains(&msr) {
            0
        } else if (0xC000_0000..=0xC000_1FFF).contains(&msr) {
            1 * RANGE_SIZE
        } else if (0xC001_0000..=0xC001_1FFF).contains(&msr) {
            2 * RANGE_SIZE
        } else {
            3 * RANGE_SIZE
        }
    }

    #[allow(unused)]
    fn set_bit(&mut self, msr: u32) {
        // https://github.com/reactos/reactos/blob/3fa57b8ff7fcee47b8e2ed869aecaf4515603f3f/sdk/lib/rtl/bitmap.c#L372-L430
        // Based on RtlSetBits

        // TODO: Find the correct range
        let _range = 0;
        let _offset = msr & 0xffff;

        // TODO: Implement this
    }
}
