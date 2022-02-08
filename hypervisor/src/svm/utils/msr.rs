use bitfield::bitfield;
use x86::msr::{rdmsr, wrmsr};

pub const SVM_MSR_TSC: u32 = 0x00000010;
pub const SVM_MSR_VM_HSAVE_PA: u32 = 0xc001_0117;
pub const EFER_SVME: u64 = 1 << 12;
pub const SVM_MSR_TSC_RATIO: u32 = 0xC000_0104;
pub const SVM_MSR_DEBUG_CTL: u32 = 0x0000_01D9;

/// Last Branch From IP
pub const MSR_BR_FROM: u32 = 0x0000_01DB;
/// Last Branch To IP
pub const MSR_BR_TO: u32 = 0x0000_01DC;
/// Last Exception From IP
pub const MSR_LAST_EXCP_FROM_IP: u32 = 0x0000_01DD;
/// Last Exception To IP
pub const MSR_LAST_EXCP_TO_IP: u32 = 0x0000_01DE;

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
    // 593 MSR0000_0000: Load-Store MCA Address
    // 593 MSR0000_0001: Load-Store MCA Status
    // 593 MSR0000_0010: Time Stamp Counter (TSC)
    // 593 MSR0000_001B: APIC Base Address (APIC_BAR)
    // 594 MSR0000_002A: Cluster ID (EBL_CR_POWERON)
    // 594 MSR0000_008B: Patch Level (PATCH_LEVEL)
    // 594 MSR0000_00E7: Max Performance Frequency Clock Count (MPERF)
    // 594 MSR0000_00E8: Actual Performance Frequency Clock Count (APERF)
    // 594 MSR0000_00FE: MTRR Capabilities (MTRRcap)
    // 595 MSR0000_0174: SYSENTER CS (SYSENTER_CS)
    // 595 MSR0000_0175: SYSENTER ESP (SYSENTER_ESP)
    // 595 MSR0000_0176: SYSENTER EIP (SYSENTER_EIP)
    // 595 MSR0000_0179: Global Machine Check Capabilities (MCG_CAP)
    // 595 MSR0000_017A: Global Machine Check Status (MCG_STAT)
    // 596 MSR0000_017B: Global Machine Check Exception Reporting Control (MCG_CTL)
    // 596 MSR0000_01D9: Debug Control (DBG_CTL_MSR)
    // 596 MSR0000_01DB: Last Branch From IP (BR_FROM)
    // 597 MSR0000_01DC: Last Branch To IP (BR_TO)
    // 597 MSR0000_01DD: Last Exception From IP
    // 597 MSR0000_01DE: Last Exception To IP
    // 597 MSR0000_020[F:0]: Variable-Size MTRRs Base/Mask
    // 600 MSR0000_02[6F:68,59:58,50]: Fixed-Size MTRRs
    // 602 MSR0000_0277: Page Attribute Table (PAT)
    // 602 MSR0000_02FF: MTRR Default Memory Type (MTRRdefType)
    // 603 MSR0000_0400: MC0 Machine Check Control (MC0_CTL)
    // 603 MSR0000_0401: MC0 Machine Check Status (MC0_STATUS)
    // 606 MSR0000_0402: MC0 Machine Check Address (MC0_ADDR)
    // 607 MSR0000_0403: MC0 Machine Check Miscellaneous (MC0_MISC)
    // 608 MSR0000_0404: MC1 Machine Check Control (MC1_CTL)
    // 609 MSR0000_0405: MC1 Machine Check Status (MC1_STATUS)
    // 613 MSR0000_0406: MC1 Machine Check Address (MC1_ADDR)
    // 615 MSR0000_0407: MC1 Machine Check Miscellaneous (MC1_MISC)
    // 616 MSR0000_0408: MC2 Machine Check Control (MC2_CTL)
    // 617 MSR0000_0409: MC2 Machine Check Status (MC2_STATUS)
    // 621 MSR0000_040A: MC2 Machine Check Address (MC2_ADDR)
    // 622 MSR0000_040B: MC2 Machine Check Miscellaneous (MC2_MISC)
    // 623 MSR0000_040C: MC3 Machine Check Control (MC3_CTL)
    // 623 MSR0000_040D: MC3 Machine Check Status (MC3_STATUS)
    // 623 MSR0000_040E: MC3 Machine Check Address (MC3_ADDR)
    // 623 MSR0000_040F: MC3 Machine Check Miscellaneous (MC3_MISC)
    // 623 MSR0000_0410: MC4 Machine Check Control (MC4_CTL)
    // 624 MSR0000_0411: MC4 Machine Check Status (MC4_STATUS)
    // 627 MSR0000_0412: MC4 Machine Check Address (MC4_ADDR)
    // 628 MSR0000_0413: NB Machine Check Misc 4 (MC4_MISC0)
    // 628 MSR0000_0414: MC5 Machine Check Control (MC5_CTL)
    // 629 MSR0000_0415: MC5 Machine Check Status (MC5_STATUS)
    // 631 MSR0000_0416: MC5 Machine Check Address (MC5_ADDR)
    // 632 MSR0000_0417: MC5 Machine Check Miscellaneous (MC5_MISC)
    // 632 MSR0000_0418: MC6 Machine Check Control (MC6_CTL)
    // 633 MSR0000_0419: MC6 Machine Check Status (MC6_STATUS)
    // 634 MSR0000_041A: MC6 Machine Check Address (MC6_ADDR)
    // 634 MSR0000_041B: MC6 Machine Check Miscellaneous (MC6_MISC)
    // 635 MSRC000_0080: Extended Feature Enable (EFER)
    // 635 MSRC000_0081: SYSCALL Target Address (STAR)
    // 635 MSRC000_0082: Long Mode SYSCALL Target Address (STAR64)
    // 636 MSRC000_0083: Compatibility Mode SYSCALL Target Address (STARCOMPAT)
    // 636 MSRC000_0084: SYSCALL Flag Mask (SYSCALL_FLAG_MASK)
    // 636 MSRC000_00E7: Read-Only Max Performance Frequency Clock Count
    // (MPerfReadOnly) 636 MSRC000_00E8: Read-Only Actual Performance Frequency
    // Clock Count (APerfReadOnly) 637 MSRC000_0100: FS Base (FS_BASE)
    // 637 MSRC000_0101: GS Base (GS_BASE)
    // 637 MSRC000_0102: Kernel GS Base (KernelGSbase)
    // 637 MSRC000_0103: Auxiliary Time Stamp Counter (TSC_AUX)
    // 637 MSRC000_0104: Time Stamp Counter Ratio (TscRateMsr)
    // 638 MSRC000_0105: Lightweight Profile Configuration (LWP_CFG)
    // 639 MSRC000_0106: Lightweight Profile Control Block Address (LWP_CBADDR)
    // 639 MSRC000_0408: NB Machine Check Misc 4 (Link Thresholding) 1 (MC4_MISC1)
    // 640 MSRC000_0409: NB Machine Check Misc 4 (L3 Thresholding) 1 (MC4_MISC2)
    // 641 MSRC000_040[F:A]: Reserved
    // 641 MSRC000_0410: Machine Check Deferred Error Configuration (CU_DEFER_ERR)
    // 642 MSRC001_00[03:00]: Performance Event Select (PERF_CTL[3:0])
    // 642 MSRC001_00[07:04]: Performance Event Counter (PERF_CTR[3:0])
    // 642 MSRC001_0010: System Configuration (SYS_CFG)
    // 643 MSRC001_0015: Hardware Configuration (HWCR)
    // 645 MSRC001_00[18,16]: IO Range Base (IORR_BASE[1:0])
    // 646 MSRC001_00[19,17]: IO Range Mask (IORR_MASK[1:0])
    // 646 MSRC001_001A: Top Of Memory (TOP_MEM)
    // 646 MSRC001_001D: Top Of Memory 2 (TOM2)
    // 646 MSRC001_001F: Northbridge Configuration 1 (NB_CFG1)
    // 647 MSRC001_0022: Machine Check Exception Redirection
    // 647 MSRC001_00[35:30]: Processor Name String
    // 648 MSRC001_003E: Hardware Thermal Control (HTC)
    // 648 MSRC001_0044: DC Machine Check Control Mask (MC0_CTL_MASK)
    // 648 MSRC001_0045: IC Machine Check Control Mask (MC1_CTL_MASK)
    // 649 MSRC001_0046: BU Machine Check Control Mask (MC2_CTL_MASK)
    // 650 MSRC001_0047: Reserved (MC3_CTL_MASK)
    // 650 MSRC001_0048: NB Machine Check Control Mask (MC4_CTL_MASK)
    // 650 MSRC001_0049: EX Machine Check Control Mask (MC5_CTL_MASK)
    // 650 MSRC001_004A: FP Machine Check Control Mask (MC6_CTL_MASK)
    // 650 MSRC001_00[53:50]: IO Trap (SMI_ON_IO_TRAP_[3:0])
    // 651 MSRC001_0054: IO Trap Control (SMI_ON_IO_TRAP_CTL_STS)
    // 652 MSRC001_0055: Interrupt Pending
    // 653 MSRC001_0056: SMI Trigger IO Cycle
    // 653 MSRC001_0058: MMIO Configuration Base Address
    // 654 MSRC001_0060: BIST Results
    // 654 MSRC001_0061: P-state Current Limit
    // 654 MSRC001_0062: P-state Control
    // 655 MSRC001_0063: P-state Status
    // 655 MSRC001_00[6B:64]: P-state [7:0]
    // 656 MSRC001_0070: COFVID Control
    // 657 MSRC001_0071: COFVID Status
    // 657 MSRC001_0073: C-state Base Address
    // 657 MSRC001_0074: CPU Watchdog Timer (CpuWdtCfg)
    // 658 MSRC001_007A: Compute Unit Power Accumulator
    // 658 MSRC001_007B: Max Compute Unit Power Accumulator
    // 658 MSRC001_0111: SMM Base Address (SMM_BASE)
    // 659 MSRC001_0112: SMM TSeg Base Address (SMMAddr)
    // 659 MSRC001_0113: SMM TSeg Mask (SMMMask)
    // 660 MSRC001_0114: Virtual Machine Control (VM_CR)
    // 661 MSRC001_0115: IGNNE
    // 661 MSRC001_0116: SMM Control (SMM_CTL)
    // 662 MSRC001_0117: Virtual Machine Host Save Physical Address (VM_HSAVE_PA)
    // 662 MSRC001_0118: SVM Lock Key
    // 662 MSRC001_011A: Local SMI Status
    // 663 MSRC001_0140: OS Visible Work-around MSR0 (OSVW_ID_Length)
    // 663 MSRC001_0141: OS Visible Work-around MSR1 (OSVW Status)
    // 663 MSRC001_020[A,8,6,4,2,0]: Performance Event Select (PERF_CTL[5:0])
    // 665 MSRC001_020[B,9,7,5,3,1]: Performance Event Counter (PERF_CTR[5:0])
    // 665 MSRC001_024[6,4,2,0]: Northbridge Performance Event Select
    // (NB_PERF_CTL[3:0]) 666 MSRC001_024[7,5,3,1]: Northbridge Performance
    // Event Counter (NB_PERF_CTR[3:0]) 666 MSRC001_0280: Performance Time Stamp
    // Counter (CU_PTSC) 667 MSRC001_1002: CPUID Features for CPUID
    // Fn0000_0007_E[B,A]X_x0 667 MSRC001_1003: Thermal and Power Management
    // CPUID Features 667 MSRC001_1004: CPUID Features (Features)
    // 669 MSRC001_1005: Extended CPUID Features (ExtFeatures)
    // 671 MSRC001_101[B:9]: Address Mask For DR[3:1] Breakpoints
    // 671 MSRC001_101C: Load-Store Configuration 3 (LS_CFG3)
    // 672 MSRC001_1020: Load-Store Configuration (LS_CFG)
    // 672 MSRC001_1021: Instruction Cache Configuration (IC_CFG)
    // 672 MSRC001_1022: Data Cache Configuration (DC_CFG)
    // 672 MSRC001_1023: Combined Unit Configuration (CU_CFG)
    // 673 MSRC001_1027: Address Mask For DR0 Breakpoints (DR0_ADDR_MASK)
    // 673 MSRC001_1028: Floating Point Configuration (FP_CFG)
    // 673 MSRC001_102A: Combined Unit Configuration 2 (CU_CFG2)
    // 675 MSRC001_102B: Combined Unit Configuration 3 (CU_CFG3)
    // 676 MSRC001_102F: Prefetch Throttling Configuration (CU_PFTCFG)
    // 677 MSRC001_1030: IBS Fetch Control (IbsFetchCtl)
    // 678 MSRC001_1031: IBS Fetch Linear Address (IbsFetchLinAd)
    // 679 MSRC001_1032: IBS Fetch Physical Address (IbsFetchPhysAd)
    // 679 MSRC001_1033: IBS Execution Control (IbsOpCtl)
    // 680 MSRC001_1034: IBS Op Logical Address (IbsOpRip)
    // 680 MSRC001_1035: IBS Op Data (IbsOpData)
    // 681 MSRC001_1036: IBS Op Data 2 (IbsOpData2)
    // 681 MSRC001_1037: IBS Op Data 3 (IbsOpData3)
    // 683 MSRC001_1038: IBS DC Linear Address (IbsDcLinAd)
    // 683 MSRC001_1039: IBS DC Physical Address (IbsDcPhysAd)
    // 683 MSRC001_103A: IBS Control
    // 683 MSRC001_103B: IBS Branch Target Address (BP_IBSTGT_RIP)
    // 684 MSRC001_103C: IBS Fetch Control Extended (IC_IBS_EXTD_CTL)
    // 684 MSRC001_103D: IBS Op Data 4 (DC_IBS_DATA2)
    // 684 MSRC001_1090: Processor Feedback Constants 0
    // 684 MSRC001_10A1: Contention Blocking Buffer Control (CU_CBBCFG)

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

pub fn toggle_debug_control(enable: bool) {
    bitfield! {
        pub struct DebugCtl(u64);

        pub last_branch_record, set_last_branch_record: 0, 0;
        pub branch_single_step, set_branch_single_step: 1, 1;

        // Performance Monitoring Pin Control
        pub pb0, set_pb0: 2, 2;
        pub pb1, set_pb1: 3, 3;
        pub pb2, set_pb2: 4, 4;
        pub pb3, set_pb3: 5, 5;
    }

    let mut value = DebugCtl(unsafe { rdmsr(SVM_MSR_DEBUG_CTL) });
    value.set_last_branch_record(enable as u64);
    unsafe { wrmsr(SVM_MSR_DEBUG_CTL, value.0) };
}
