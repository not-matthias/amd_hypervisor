//! Checks whether there's a hypervisor installed.
//!
//! References:
//! - https://github.com/LazyAhora/Hypervisor_detect_ring_0

use hypervisor::svm::msr::{EFER_SVME, SVM_MSR_TSC, SVM_MSR_VM_HSAVE_PA};
use x86::{
    cpuid::cpuid,
    msr::{rdmsr, IA32_EFER},
    time::rdtsc,
};
use x86_64::instructions::interrupts::without_interrupts;

#[derive(Debug)]
pub enum HvStatus {
    HypervisorPresent,
    HypervisorNotPresent,
}

pub fn check_all() {
    log::info!("Checking for Hypervisor...");

    log::info!("check_reserved_cpuid: {:?}", check_reserved_cpuid());
    log::info!("check_reserved_cpuid_2: {:?}", check_reserved_cpuid_2());
    log::info!("check_cpuid_hv: {:?}", check_cpuid_hv());
    log::info!("check_rdtsc: {:?}", check_rdtsc_cpuid_rdtsc());
    log::info!("check_rdtsc_msr: {:?}", check_rdtsc_msr());
    log::info!("check_mperf: {:?}", check_mperf());
    log::info!("check_aperf: {:?}", check_aperf());
    log::info!("check_hsave_msr: {:?}", check_hsave_msr());
    log::info!("check_efer_msr: {:?}", check_efer_msr());
}

pub fn check_reserved_cpuid() -> HvStatus {
    let invalid_cpuid = cpuid!(0x13371337);
    let reserved_cpuid = cpuid!(0x40000000);

    // The reserved cpuid is used to report vmm capabilities.
    //
    if invalid_cpuid != reserved_cpuid {
        HvStatus::HypervisorPresent
    } else {
        HvStatus::HypervisorNotPresent
    }
}

pub fn check_reserved_cpuid_2() -> HvStatus {
    // TODO: This second example uses the highest low function leaf to compare data
    // to what would be given on a real system.

    let cpuid_0 = cpuid!(0x40000000);
    let cpuid_1 = cpuid!(cpuid!(0).eax);

    if cpuid_0 != cpuid_1 {
        HvStatus::HypervisorPresent
    } else {
        HvStatus::HypervisorNotPresent
    }
}

pub fn check_cpuid_hv() -> HvStatus {
    let result = cpuid!(0x1);

    // RAZ. Reserved for use by hypervisor to indicate guest status.
    //
    if (result.ecx >> 31) & 1 == 1 {
        HvStatus::HypervisorPresent
    } else {
        HvStatus::HypervisorNotPresent
    }
}

pub fn check_rdtsc_cpuid_rdtsc() -> HvStatus {
    const RUNS: u64 = 1337;

    let mut avg = 0;
    without_interrupts(|| {
        for _i in 0..RUNS {
            let tick_1 = unsafe { rdtsc() };
            let _ = cpuid!(0x0);
            let tick_2 = unsafe { rdtsc() };

            avg += tick_2 - tick_1;
        }
    });
    avg /= RUNS;

    log::info!("Average rdtsc: {}", avg);

    if !(25..=500).contains(&avg) {
        HvStatus::HypervisorPresent
    } else {
        HvStatus::HypervisorNotPresent
    }
}

pub fn check_rdtsc_rdtsc() -> HvStatus {
    const RUNS: u64 = 1337;

    let mut avg = 0;
    without_interrupts(|| {
        for _i in 0..RUNS {
            let tick_1 = unsafe { rdtsc() };
            let tick_2 = unsafe { rdtsc() };

            avg += tick_2 - tick_1;
        }
    });
    avg /= RUNS;

    log::info!("Average rdtsc: {}", avg);

    if !(25..=500).contains(&avg) {
        HvStatus::HypervisorPresent
    } else {
        HvStatus::HypervisorNotPresent
    }
}

pub fn check_rdtsc_msr() -> HvStatus {
    const RUNS: u64 = 1337;

    let mut avg = 0;
    without_interrupts(|| {
        for _ in 0..RUNS {
            let tick_1 = unsafe { rdmsr(SVM_MSR_TSC) };
            let _ = cpuid!(0x0);
            let tick_2 = unsafe { rdmsr(SVM_MSR_TSC) };

            avg += tick_2 - tick_1;
        }
    });
    avg /= RUNS;

    log::info!("Average rdtsc_msr: {}", avg);

    if !(25..=500).contains(&avg) {
        HvStatus::HypervisorPresent
    } else {
        HvStatus::HypervisorNotPresent
    }
}

pub fn check_aperf() -> HvStatus {
    const APERF_MSR: u32 = 0xE8;
    const RUNS: u64 = 1337;

    let mut avg = 0;
    without_interrupts(|| {
        for _i in 0..RUNS {
            let tick_1 = unsafe { rdmsr(APERF_MSR) } << 32;
            let _ = cpuid!(0x0);
            let tick_2 = unsafe { rdmsr(APERF_MSR) } << 32;

            avg += tick_2 - tick_1;
        }
    });
    avg /= RUNS;

    if !(0x00000BE30000..=0x00000FFF0000000).contains(&avg) {
        log::warn!("Invalid aperf value: {:?}", avg);
        HvStatus::HypervisorPresent
    } else {
        HvStatus::HypervisorNotPresent
    }
}

pub fn check_mperf() -> HvStatus {
    const MPERF_MSR: u32 = 0xE8;
    const RUNS: u64 = 1337;

    let mut avg = 0;
    without_interrupts(|| {
        for _i in 0..RUNS {
            let tick_1 = unsafe { rdmsr(MPERF_MSR) };
            let _ = cpuid!(0x0);
            let tick_2 = unsafe { rdmsr(MPERF_MSR) };

            avg += tick_2 - tick_1;
        }
    });
    avg /= RUNS;

    if !(0xc..=0xff).contains(&avg) {
        log::warn!("Invalid mperf value: {:?}", avg);
        HvStatus::HypervisorPresent
    } else {
        HvStatus::HypervisorNotPresent
    }
}

/// Checks whether the guest has set the `host save physical address` msr. If
/// it's not set, it should just be 0.
///
/// This could be used to find the host save area.
pub fn check_hsave_msr() -> HvStatus {
    let result = unsafe { rdmsr(SVM_MSR_VM_HSAVE_PA) };
    if result != 0 {
        log::info!("Host save physical address: {:?}", result);
        HvStatus::HypervisorPresent
    } else {
        HvStatus::HypervisorNotPresent
    }
}

pub fn check_efer_msr() -> HvStatus {
    let result = unsafe { rdmsr(IA32_EFER) };
    if result & EFER_SVME != 0 {
        log::info!("SVM enabled: {:?}", result);
        HvStatus::HypervisorPresent
    } else {
        HvStatus::HypervisorNotPresent
    }
}

pub fn check_debug_ctl() {
    // 01D9h DebugCtl Software Debug “Debug-Control MSR (DebugCtl)” on page 391
    const DEBUG_CTL_MSR: u32 = 0x01D9;

    let _result = unsafe { rdmsr(DEBUG_CTL_MSR) };

    // DWORD64 current_value = __readmsr(MSR_DEBUGCTL);//safe current value
    // __writemsr(MSR_DEBUGCTL, DEBUGCTL_LBR | DEBUGCTL_BTF);
    // DWORD64 whatch_write = __readmsr(MSR_DEBUGCTL);
    // __writemsr(MSR_DEBUGCTL, current_value);
    // return (!(whatch_write & DEBUGCTL_LBR));

    // TODO: implement
}
