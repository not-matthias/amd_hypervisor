use bitfield::bitfield;
use x86::msr::{rdmsr, wrmsr};

pub const SVM_MSR_TSC: u32 = 0x00000010;
pub const SVM_MSR_VM_HSAVE_PA: u32 = 0xc001_0117;
pub const EFER_SVME: u64 = 1 << 12;
pub const SVM_MSR_TSC_RATIO: u32 = 0xC000_0104;

pub fn set_tsc_ratio(ratio: f32) {
    let integer = unsafe { ratio.to_int_unchecked::<u8>() };
    let fractional = ratio - integer as f32;

    log::info!("Setting TSC ratio to {}.{}", integer, fractional);
    log::info!("Fract bits: {:?}", fractional.to_bits());

    // 39:32 INT Integer Part
    // 31:0 FRAC Fractional Part
    //
    bitfield! {
        pub struct TscRatio(u64);

        pub frac, set_frac  : 31, 0;
        pub int, set_int    : 39, 32;
    }
    let mut value = TscRatio(0);
    value.set_frac(fractional.to_bits() as u64);
    value.set_int(integer as u64);

    log::info!("tsc_ratio: {:?}", unsafe { rdmsr(SVM_MSR_TSC_RATIO) });
    unsafe { wrmsr(SVM_MSR_TSC_RATIO, value.0) };
    log::info!("tsc_ratio: {:?}", unsafe { rdmsr(SVM_MSR_TSC_RATIO) });
}
