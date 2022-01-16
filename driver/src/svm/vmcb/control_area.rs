use bitflags::bitflags;

// Size: 0x400
#[repr(C)]
pub struct ControlArea {
    pub intercept_cr_read: u16,   // +0x000
    pub intercept_cr_write: u16,  // +0x002
    pub intercept_dr_read: u16,   // +0x004
    pub intercept_dr_write: u16,  // +0x006
    pub intercept_exception: u32, // +0x008

    pub intercept_misc1: InterceptMisc1,     // +0x00c
    pub intercept_misc2: InterceptMisc2,     // +0x010
    pub reserved1: [u8; 0x03c - 0x014],      // +0x014
    pub pause_filter_threshold: u16,         // +0x03c
    pub pause_filter_count: u16,             // +0x03e
    pub iopm_base_pa: u64,                   // +0x040
    pub msrpm_base_pa: u64,                  // +0x048
    pub tsc_offset: u64,                     // +0x050
    pub guest_asid: u32,                     // +0x058
    pub tlb_control: TlbControl,             // +0x05c
    pub vintr: u64,                          // +0x060
    pub interrupt_shadow: u64,               // +0x068
    pub exit_code: VmExitCode,               // +0x070
    pub exit_info1: NptExitInfo,             // +0x078
    pub exit_info2: u64,                     // +0x080
    pub exit_int_info: u64,                  // +0x088
    pub np_enable: NpEnable,                 // +0x090
    pub avic_apic_bar: u64,                  // +0x098
    pub guest_pa_of_ghcb: u64,               // +0x0a0
    pub event_inj: u64,                      // +0x0a8
    pub ncr3: u64,                           // +0x0b0
    pub lbr_virtualization_enable: u64,      // +0x0b8
    pub vmcb_clean: VmcbClean,               // +0x0c0
    pub nrip: u64,                           // +0x0c8
    pub num_of_bytes_fetched: u8,            // +0x0d0
    pub guest_instruction_bytes: [u8; 15],   // +0x0d1
    pub avic_apic_backing_page_pointer: u64, // +0x0e0
    pub reserved2: u64,                      // +0x0e8
    pub avic_logical_table_pointer: u64,     // +0x0f0
    pub avic_physical_table_pointer: u64,    // +0x0f8
    pub reserved3: u64,                      // +0x100
    pub vmcb_save_state_pointer: u64,        // +0x108
    pub reserved4: [u8; 0x400 - 0x110],      // +0x110
}

bitflags! {
    /// See `15.15.3 VMCB Clean Field`
    ///
    /// Bits 31:12 are reserved for future implementations. For forward compatibility, if the hypervisor has
    /// not modified the VMCB, the hypervisor may write FFFF_FFFFh to the VMCB Clean Field to indicate
    /// that it has not changed any VMCB contents other than the fields described below as explicitly
    /// uncached. **The hypervisor should write 0h to indicate that the VMCB is new or potentially inconsistent
    /// with the CPU's cached copy**, as occurs when the hypervisor has allocated a new location for an existing
    /// VMCB from a list of free pages and does not track whether that page had recently been used as a
    /// VMCB for another guest. If any VMCB fields (excluding explicitly uncached fields) have been
    /// modified, all clean bits that are undefined (within the scope of the hypervisor) must be cleared to zero.
    pub struct VmcbClean: u64 {
        /// Intercepts: all the intercept vectors, TSC offset, Pause Filter Count
        const I = 1 << 0;

        /// IOMSRPM: IOPM_BASE, MSRPM_BASE
        const IOPM = 1 << 1;

        /// ASID
        const ASID = 1 << 2;

        /// V_TPR, V_IRQ, V_INTR_PRIO, V_IGN_TPR, V_INTR_MASKING, V_INTR_VECTOR (Offset 60h–67h)
        const TPR = 1 << 3;

        /// Nested Paging: NCR3, G_PAT
        const NP = 1 << 4;

        /// CR0, CR3, CR4, EFER
        const CR_X = 1 << 5;

        /// DR6, DR7
        const DR_X = 1 << 6;

        /// GDT/IDT Limit and Base
        const DT = 1 << 7;

        /// CS/DS/SS/ES Sel/Base/Limit/Attr, CPL
        const SEG = 1 << 8;

        /// CR2
        const CR2 = 1 << 9;

        /// DbgCtlMsr, br_from/to, lastint_from/to
        const LBR = 1 << 10;

        /// AVIC APIC_BAR; AVIC APIC_BACKING_PAGE, AVIC PHYSICAL_TABLE and AVIC LOGICAL_
        /// TABLE Pointers
        const AVIC = 1 << 11;

        /// S_CET, SSP, ISST_ADDR
        const CET = 1 << 12;
    }

    pub struct TlbControl: u32 {
        /// 00h—Do nothing.
        const DO_NOTHING                        = 0;

        /// 01h—Flush entire TLB (all entries, all ASIDs) on VMRUN.
        /// Should only be used by legacy hypervisors.
        const FLUSH_ENTIRE_TLB                  = 1;

        /// 03h—Flush this guest’s TLB entries.
        const FLUSH_GUEST_TLB                   = 3;

        /// 07h—Flush this guest’s non-global TLB entries.
        const FLUSH_GUEST_NON_GLOBAL_TLB        = 4;
    }

    pub struct NpEnable: u64 {
        const NESTED_PAGING                     = 1 << 0;
        const SECURE_ENCRYPTED_VIRTUALIZATION   = 1 << 1;
        const ENCRYPTED_STATE                   = 1 << 2;
        const GUEST_MODE_EXECUTE_TRAP           = 1 << 3;
        const SSS_CHECK_EN                      = 1 << 4;
        const VIRTUAL_TRANSPARENT_ENCRYPTION    = 1 << 5;
        const ENABLE_INVLPGB                    = 1 << 7;
    }

    pub struct InterceptMisc1: u32 {
        const INTERCEPT_INTR = 1 << 0;
        const INTERCEPT_NMI = 1 << 1;
        const INTERCEPT_SMI = 1 << 2;
        const INTERCEPT_INIT = 1 << 3;
        const INTERCEPT_VINTR = 1 << 4;
        const INTERCEPT_CR0 = 1 << 5;

        const INTERCEPT_READ_IDTR = 1 << 6;
        const INTERCEPT_READ_GDTR = 1 << 7;
        const INTERCEPT_READ_LDTR = 1 << 8;
        const INTERCEPT_READ_TR = 1 << 9;

        const INTERCEPT_WRITE_IDTR = 1 << 10;
        const INTERCEPT_WRITE_GDTR = 1 << 11;
        const INTERCEPT_WRITE_LDTR = 1 << 12;
        const INTERCEPT_WRITE_TR = 1 << 13;

        const INTERCEPT_RDTSC = 1 << 14;
        const INTERCEPT_RDPMC = 1 << 15;
        const INTERCEPT_PUSHF = 1 << 16;
        const INTERCEPT_POPF = 1 << 17;
        const INTERCEPT_CPUID = 1 << 18;
        const INTERCEPT_RSM = 1 << 19;
        const INTERCEPT_IRET = 1 << 20;
        const INTERCEPT_INTN = 1 << 21;
        const INTERCEPT_INVD = 1 << 22;
        const INTERCEPT_PAUSE = 1 << 23;
        const INTERCEPT_HLT = 1 << 24;
        const INTERCEPT_INVLPG = 1 << 25;
        const INTERCEPT_INVLPGA = 1 << 26;
        const INTERCEPT_IOIO_PROT = 1 << 27;
        const INTERCEPT_MSR_PROT = 1 << 28;
        const INTERCEPT_TASK_SWITCHES = 1 << 29;
        const INTERCEPT_FERR_FREEZE = 1 << 30;
        const INTERCEPT_SHUTDOWN = 1 << 31;
    }

    pub struct InterceptMisc2: u32 {
        const INTERCEPT_VMRUN = 1 << 0;
        const INTERCEPT_VMCALL = 1 << 1;
        const INTERCEPT_VMLOAD = 1 << 2;
        const INTERCEPT_VMSAVE = 1 << 3;
        const INTERCEPT_STGI = 1 << 4;
        const INTERCEPT_CLGI = 1 << 5;
        const INTERCEPT_SKINIT = 1 << 6;
        const INTERCEPT_RDTSCP = 1 << 7;
        const INTERCEPT_ICEBP = 1 << 8;
        const INTERCEPT_WBINVD = 1 << 9;
        const INTERCEPT_MONITOR = 1 << 10;
        const INTERCEPT_MWAIT = 1 << 11;
        const INTERCEPT_MWAIT_CONDITIONAL = 1 << 12;
        const INTERCEPT_XSETBV = 1 << 13;
        const INTERCEPT_RDPRU = 1 << 14;
        const INTERCEPT_EFER = 1 << 15;
        const INTERCEPT_CR0 = 1 << 16;
        const INTERCEPT_CR1 = 1 << 17;
        const INTERCEPT_CR2 = 1 << 18;
        const INTERCEPT_CR3 = 1 << 19;
        const INTERCEPT_CR4 = 1 << 20;
        const INTERCEPT_CR5 = 1 << 21;
        const INTERCEPT_CR6 = 1 << 22;
        const INTERCEPT_CR7 = 1 << 23;
        const INTERCEPT_CR8 = 1 << 24;
        const INTERCEPT_CR9 = 1 << 25;
        const INTERCEPT_CR10 = 1 << 26;
        const INTERCEPT_CR11 = 1 << 27;
        const INTERCEPT_CR12 = 1 << 28;
        const INTERCEPT_CR13 = 1 << 29;
        const INTERCEPT_CR14 = 1 << 30;
        const INTERCEPT_CR15 = 1 << 31;
    }

    pub struct VmExitCode: u64  {
        const VMEXIT_CR0_READ = 0;
        const VMEXIT_CR1_READ = 1;
        const VMEXIT_CR2_READ = 2;
        const VMEXIT_CR3_READ = 3;
        const VMEXIT_CR4_READ = 4;
        const VMEXIT_CR5_READ = 5;
        const VMEXIT_CR6_READ = 6;
        const VMEXIT_CR7_READ = 7;
        const VMEXIT_CR8_READ = 8;
        const VMEXIT_CR9_READ = 9;
        const VMEXIT_CR10_READ = 10;
        const VMEXIT_CR11_READ = 11;
        const VMEXIT_CR12_READ = 12;
        const VMEXIT_CR13_READ = 13;
        const VMEXIT_CR14_READ = 14;
        const VMEXIT_CR15_READ = 15;
        const VMEXIT_CR0_WRITE = 16;
        const VMEXIT_CR1_WRITE = 17;
        const VMEXIT_CR2_WRITE = 18;
        const VMEXIT_CR3_WRITE = 19;
        const VMEXIT_CR4_WRITE = 20;
        const VMEXIT_CR5_WRITE = 21;
        const VMEXIT_CR6_WRITE = 22;
        const VMEXIT_CR7_WRITE = 23;
        const VMEXIT_CR8_WRITE = 24;
        const VMEXIT_CR9_WRITE = 25;
        const VMEXIT_CR10_WRITE = 26;
        const VMEXIT_CR11_WRITE = 27;
        const VMEXIT_CR12_WRITE = 28;
        const VMEXIT_CR13_WRITE = 29;
        const VMEXIT_CR14_WRITE = 30;
        const VMEXIT_CR15_WRITE = 31;
        const VMEXIT_DR0_READ = 32;
        const VMEXIT_DR1_READ = 33;
        const VMEXIT_DR2_READ = 34;
        const VMEXIT_DR3_READ = 35;
        const VMEXIT_DR4_READ = 36;
        const VMEXIT_DR5_READ = 37;
        const VMEXIT_DR6_READ = 38;
        const VMEXIT_DR7_READ = 39;
        const VMEXIT_DR8_READ = 40;
        const VMEXIT_DR9_READ = 41;
        const VMEXIT_DR10_READ = 42;
        const VMEXIT_DR11_READ = 43;
        const VMEXIT_DR12_READ = 44;
        const VMEXIT_DR13_READ = 45;
        const VMEXIT_DR14_READ = 46;
        const VMEXIT_DR15_READ = 47;
        const VMEXIT_DR0_WRITE = 48;
        const VMEXIT_DR1_WRITE = 49;
        const VMEXIT_DR2_WRITE = 50;
        const VMEXIT_DR3_WRITE = 51;
        const VMEXIT_DR4_WRITE = 52;
        const VMEXIT_DR5_WRITE = 53;
        const VMEXIT_DR6_WRITE = 54;
        const VMEXIT_DR7_WRITE = 55;
        const VMEXIT_DR8_WRITE = 56;
        const VMEXIT_DR9_WRITE = 57;
        const VMEXIT_DR10_WRITE = 58;
        const VMEXIT_DR11_WRITE = 59;
        const VMEXIT_DR12_WRITE = 60;
        const VMEXIT_DR13_WRITE = 61;
        const VMEXIT_DR14_WRITE = 62;
        const VMEXIT_DR15_WRITE = 63;
        const VMEXIT_EXCEPTION_DE = 64;
        const VMEXIT_EXCEPTION_DB = 65;
        const VMEXIT_EXCEPTION_NMI = 66;
        const VMEXIT_EXCEPTION_BP = 67;
        const VMEXIT_EXCEPTION_OF = 68;
        const VMEXIT_EXCEPTION_BR = 69;
        const VMEXIT_EXCEPTION_UD = 70;
        const VMEXIT_EXCEPTION_NM = 71;
        const VMEXIT_EXCEPTION_DF = 72;
        const VMEXIT_EXCEPTION_09 = 73;
        const VMEXIT_EXCEPTION_TS = 74;
        const VMEXIT_EXCEPTION_NP = 75;
        const VMEXIT_EXCEPTION_SS = 76;
        const VMEXIT_EXCEPTION_GP = 77;
        const VMEXIT_EXCEPTION_PF = 78;
        const VMEXIT_EXCEPTION_15 = 79;
        const VMEXIT_EXCEPTION_MF = 80;
        const VMEXIT_EXCEPTION_AC = 81;
        const VMEXIT_EXCEPTION_MC = 82;
        const VMEXIT_EXCEPTION_XF = 83;
        const VMEXIT_EXCEPTION_20 = 84;
        const VMEXIT_EXCEPTION_21 = 85;
        const VMEXIT_EXCEPTION_22 = 86;
        const VMEXIT_EXCEPTION_23 = 87;
        const VMEXIT_EXCEPTION_24 = 88;
        const VMEXIT_EXCEPTION_25 = 89;
        const VMEXIT_EXCEPTION_26 = 90;
        const VMEXIT_EXCEPTION_27 = 91;
        const VMEXIT_EXCEPTION_28 = 92;
        const VMEXIT_EXCEPTION_VC = 93;
        const VMEXIT_EXCEPTION_SX = 94;
        const VMEXIT_EXCEPTION_31 = 95;
        const VMEXIT_INTR = 96;
        const VMEXIT_NMI = 97;
        const VMEXIT_SMI = 98;
        const VMEXIT_INIT = 99;
        const VMEXIT_VINTR = 100;
        const VMEXIT_CR0_SEL_WRITE = 101;
        const VMEXIT_IDTR_READ = 102;
        const VMEXIT_GDTR_READ = 103;
        const VMEXIT_LDTR_READ = 104;
        const VMEXIT_TR_READ = 105;
        const VMEXIT_IDTR_WRITE = 106;
        const VMEXIT_GDTR_WRITE = 107;
        const VMEXIT_LDTR_WRITE = 108;
        const VMEXIT_TR_WRITE = 109;
        const VMEXIT_RDTSC = 110;
        const VMEXIT_RDPMC = 111;
        const VMEXIT_PUSHF = 112;
        const VMEXIT_POPF = 113;
        const VMEXIT_CPUID = 114;
        const VMEXIT_RSM = 115;
        const VMEXIT_IRET = 116;
        const VMEXIT_SWINT = 117;
        const VMEXIT_INVD = 118;
        const VMEXIT_PAUSE = 119;
        const VMEXIT_HLT = 120;
        const VMEXIT_INVLPG = 121;
        const VMEXIT_INVLPGA = 122;
        const VMEXIT_IOIO = 123;
        const VMEXIT_MSR = 124;
        const VMEXIT_TASK_SWITCH = 125;
        const VMEXIT_FERR_FREEZE = 126;
        const VMEXIT_SHUTDOWN = 127;
        const VMEXIT_VMRUN = 128;
        const VMEXIT_VMMCALL = 129;
        const VMEXIT_VMLOAD = 130;
        const VMEXIT_VMSAVE = 131;
        const VMEXIT_STGI = 132;
        const VMEXIT_CLGI = 133;
        const VMEXIT_SKINIT = 134;
        const VMEXIT_RDTSCP = 135;
        const VMEXIT_ICEBP = 136;
        const VMEXIT_WBINVD = 137;
        const VMEXIT_MONITOR = 138;
        const VMEXIT_MWAIT = 139;
        const VMEXIT_MWAIT_CONDITIONAL = 140;
        const VMEXIT_XSETBV = 141;
        const VMEXIT_EFER_WRITE_TRAP = 143;
        const VMEXIT_CR0_WRITE_TRAP = 144;
        const VMEXIT_CR1_WRITE_TRAP = 145;
        const VMEXIT_CR2_WRITE_TRAP = 146;
        const VMEXIT_CR3_WRITE_TRAP = 147;
        const VMEXIT_CR4_WRITE_TRAP = 148;
        const VMEXIT_CR5_WRITE_TRAP = 149;
        const VMEXIT_CR6_WRITE_TRAP = 150;
        const VMEXIT_CR7_WRITE_TRAP = 151;
        const VMEXIT_CR8_WRITE_TRAP = 152;
        const VMEXIT_CR9_WRITE_TRAP = 153;
        const VMEXIT_CR10_WRITE_TRAP = 154;
        const VMEXIT_CR11_WRITE_TRAP = 155;
        const VMEXIT_CR12_WRITE_TRAP = 156;
        const VMEXIT_CR13_WRITE_TRAP = 157;
        const VMEXIT_CR14_WRITE_TRAP = 158;
        const VMEXIT_CR15_WRITE_TRAP = 159;
        const VMEXIT_NPF = 1024;
        const AVIC_INCOMPLETE_IPI = 1025;
        const AVIC_NOACCEL = 1026;
        const VMEXIT_VMGEXIT = 1027;
        const VMEXIT_INVALID = u64::MAX;
    }

    /// See "Nested versus Guest Page Faults, Fault Ordering"
    pub struct NptExitInfo: u64 {
        /// Bit 0 (P)—cleared to 0 if the nested page was not present, 1 otherwise
        const PRESENT           = 1 << 0;

        /// Bit 1 (RW)—set to 1 if the nested page table level access was a write. Note that host table walks for
        /// guest page tables are always treated as data writes.
        const RW                = 1 << 1;

        /// Bit 2 (US)—set to 1 if the nested page table level access was a user access. Note that nested page
        /// table accesses performed by the MMU are treated as user accesses unless there are features
        /// enabled that override this.
        const US                = 1 << 2;

        /// Bit 3 (RSV)—set to 1 if reserved bits were set in the corresponding nested page table entry
        const RSV               = 1 << 3;

        /// Bit 4 (ID)—set to 1 if the nested page table level access was a code read. Note that nested table
        /// walks for guest page tables are always treated as data writes, even if the access itself is a code read
        const ID                = 1 << 4;

        /// Bit 6 (SS) - set to 1 if the fault was caused by a shadow stack access.
        const SS                = 1 << 6;

        /// Bit 32—set to 1 if nested page fault occurred while translating the guest’s final physical address
        const GUEST_PA                    = 1 << 32;

        /// Bit 33—set to 1 if nested page fault occurred while translating the guest page tables
        const GUEST_PAGE_TABLES           = 1 << 33;

        /// Bit 37—set to 1 if the page was marked as a supervisor shadow stack page in the leaf node of the
        /// nested page table and the shadow stack check feature is enabled in VMCB offset 90h.
        const GUEST_PAGE_TABLES_WITH_SS   = 1 << 37;
    }
}
