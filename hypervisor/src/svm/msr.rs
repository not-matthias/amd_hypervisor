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

/// Checks whether the specified MSR is in the [Open-Source Register Reference for AMD CPUs](https://developer.amd.com/wp-content/resources/56255_3_03.PDF). See Page 260, `Memory Map - MSR`.
///
/// See also: `MSR Cross-Reference` in the AMD64 Architecture Programmer’s
/// Manual Volume 2:System Programming. TODO: Extract and compare from there
pub fn is_valid_msr(msr: u32) -> bool {
    // MSRs - MSR0000_xxxx
    (0x0000_0000..=0x0000_0001).contains(&msr)
        || (0x0000_0010..=0x0000_02FF).contains(&msr)
        || (0x0000_0400..=0x0000_0403).contains(&msr)
        || (0x0000_0404..=0x0000_0407).contains(&msr)
        || (0x0000_0408..=0x0000_040B).contains(&msr)
        || (0x0000_040C..=0x0000_040F).contains(&msr)
        || (0x0000_0414..=0x0000_0417).contains(&msr)
        || (0x0000_0418..=0x0000_041B).contains(&msr)
        || (0x0000_041C..=0x0000_043B).contains(&msr)
        || (0x0000_043C..=0x0000_0443).contains(&msr)
        || (0x0000_044C..=0x0000_044F).contains(&msr)
        || (0x0000_0450..=0x0000_0457).contains(&msr)
        || (0x0000_0458..=0x0000_045B).contains(&msr)
        // MSRs - MSRC000_0xxx
        || (0xC000_0080..=0xC000_0410).contains(&msr)
        || (0xC000_2000..=0xC000_2009).contains(&msr)
        || (0xC000_2010..=0xC000_2016).contains(&msr)
        || (0xC000_2020..=0xC000_2029).contains(&msr)
        || (0xC000_2030..=0xC000_2036).contains(&msr)
        || (0xC000_2040..=0xC000_2049).contains(&msr)
        || (0xC000_2050..=0xC000_2056).contains(&msr)
        || (0xC000_2060..=0xC000_2066).contains(&msr)
        || (0xC000_2070..=0xC000_20E9).contains(&msr)
        || (0xC000_20F0..=0xC000_210A).contains(&msr)
        || (0xC000_2130..=0xC000_2136).contains(&msr)
        || (0xC000_2140..=0xC000_2159).contains(&msr)
        || (0xC000_2160..=0xC000_2169).contains(&msr)
        // MSRs - MSRC001_0xxx
        || (0xC001_0000..=0xC001_029B).contains(&msr)
        || (0xC001_0400..=0xC001_0406).contains(&msr)
        || (0xC001_0407..=0xC001_040E).contains(&msr)
        || (0xC001_0413..=0xC001_0416).contains(&msr)
        // MSRs - MSRC001_1xxx
        || (0xC001_1002..=0xC001_103C).contains(&msr)
}
