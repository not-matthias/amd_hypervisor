use bitflags::bitflags;

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
    pub tlb_control: u32,                    // +0x05c
    pub vintr: u64,                          // +0x060
    pub interrupt_shadow: u64,               // +0x068
    pub exit_code: u64,                      // +0x070
    pub exit_info1: u64,                     // +0x078
    pub exit_info2: u64,                     // +0x080
    pub exit_int_info: u64,                  // +0x088
    pub np_enable: u64,                      // +0x090
    pub avic_apic_bar: u64,                  // +0x098
    pub guest_pa_of_ghcb: u64,               // +0x0a0
    pub event_inj: u64,                      // +0x0a8
    pub ncr3: u64,                           // +0x0b0
    pub lbr_virtualization_enable: u64,      // +0x0b8
    pub vmcb_clean: u64,                     // +0x0c0
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

// TODO: Test size = 0x400

bitflags! {
    pub struct InterceptMisc1: u32 {
        const INTERCEPT_INTR = 0;
        const INTERCEPT_NMI = 1;
        const INTERCEPT_SMI = 2;
        const INTERCEPT_INIT = 3;
        const INTERCEPT_VINTR = 4;
        const INTERCEPT_CR0 = 5;

        const INTERCEPT_READ_IDTR = 6;
        const INTERCEPT_READ_GDTR = 7;
        const INTERCEPT_READ_LDTR = 8;
        const INTERCEPT_READ_TR = 9;

        const INTERCEPT_WRITE_IDTR = 10;
        const INTERCEPT_WRITE_GDTR = 11;
        const INTERCEPT_WRITE_LDTR = 12;
        const INTERCEPT_WRITE_TR = 13;

        const INTERCEPT_RDTSC = 14;
        const INTERCEPT_RDPMC = 15;
        const INTERCEPT_PUSHF = 16;
        const INTERCEPT_POPF = 17;
        const INTERCEPT_CPUID = 18;
        const INTERCEPT_RSM = 19;
        const INTERCEPT_IRET = 20;
        const INTERCEPT_INTN = 21;
        const INTERCEPT_INVD = 22;
        const INTERCEPT_PAUSE = 23;
        const INTERCEPT_HLT = 24;
        const INTERCEPT_INVLPG = 25;
        const INTERCEPT_INVLPGA = 26;
        const INTERCEPT_IOIO_PROT = 27;
        const INTERCEPT_MSR_PROT = 28;
        const INTERCEPT_TASK_SWITCHES = 29;
        const INTERCEPT_FERR_FREEZE = 30;
        const INTERCEPT_SHUTDOWN = 31;
    }


    pub struct InterceptMisc2: u32 {
        const INTERCEPT_VMRUN = 0;
        const INTERCEPT_VMCALL = 1;
        const INTERCEPT_VMLOAD = 2;
        const INTERCEPT_VMSAVE = 3;
        const INTERCEPT_STGI = 4;
        const INTERCEPT_CLGI = 5;
        const INTERCEPT_SKINIT = 6;
        const INTERCEPT_RDTSCP = 7;
        const INTERCEPT_ICEBP = 8;
        const INTERCEPT_WBINVD = 9;
        const INTERCEPT_MONITOR = 10;
        const INTERCEPT_MWAIT = 11;
        const INTERCEPT_MWAIT_CONDITIONAL = 12;
        const INTERCEPT_XSETBV = 13;
        const INTERCEPT_RDPRU = 14;
        const INTERCEPT_EFER = 15;
        const INTERCEPT_CR0 = 16;
        const INTERCEPT_CR1 = 17;
        const INTERCEPT_CR2 = 18;
        const INTERCEPT_CR3 = 19;
        const INTERCEPT_CR4 = 20;
        const INTERCEPT_CR5 = 21;
        const INTERCEPT_CR6 = 22;
        const INTERCEPT_CR7 = 23;
        const INTERCEPT_CR8 = 24;
        const INTERCEPT_CR9 = 25;
        const INTERCEPT_CR10 = 26;
        const INTERCEPT_CR11 = 27;
        const INTERCEPT_CR12 = 28;
        const INTERCEPT_CR13 = 29;
        const INTERCEPT_CR14 = 30;
        const INTERCEPT_CR15 = 31;
    }
}
