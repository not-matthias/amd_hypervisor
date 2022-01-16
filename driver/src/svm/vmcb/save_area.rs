use crate::nt::include::Context;

use core::arch::asm;
use x86::controlregs::{cr2, cr3};

use x86::msr::{rdmsr, IA32_EFER, IA32_PAT};

use crate::svm::data::segmentation::{SegmentAttribute, SegmentDescriptor};

use x86_64::instructions::tables::{sgdt, sidt};
use x86_64::registers::control::{Cr0, Cr4};

// Size: 0x298
#[repr(C)]
pub struct SaveArea {
    pub es_selector: u16,
    pub es_attrib: u16,
    pub es_limit: u32,
    pub es_base: u64,

    pub cs_selector: u16,
    pub cs_attrib: u16,
    pub cs_limit: u32,
    pub cs_base: u64,

    pub ss_selector: u16,
    pub ss_attrib: u16,
    pub ss_limit: u32,
    pub ss_base: u64,

    pub ds_selector: u16,
    pub ds_attrib: u16,
    pub ds_limit: u32,
    pub ds_base: u64,

    pub fs_selector: u16,
    pub fs_attrib: u16,
    pub fs_limit: u32,
    pub fs_base: u64,

    pub gs_selector: u16,
    pub gs_attrib: u16,
    pub gs_limit: u32,
    pub gs_base: u64,

    pub gdtr_selector: u16,
    pub gdtr_attrib: u16,
    pub gdtr_limit: u32,
    pub gdtr_base: u64,

    pub ldtr_selector: u16,
    pub ldtr_attrib: u16,
    pub ldtr_limit: u32,
    pub ldtr_base: u64,

    pub idtr_selector: u16,
    pub idtr_attrib: u16,
    pub idtr_limit: u32,
    pub idtr_base: u64,

    pub tr_selector: u16,
    pub tr_attrib: u16,
    pub tr_limit: u32,
    pub tr_base: u64,

    pub reserved1: [u8; 43],
    pub cpl: u8,
    pub reserved2: u32,
    pub efer: u64,
    pub reserved3: [u8; 112],
    pub cr4: u64,
    pub cr3: u64,
    pub cr0: u64,
    pub dr7: u64,
    pub dr6: u64,
    pub rflags: u64,
    pub rip: u64,
    pub reserved4: [u8; 88],
    pub rsp: u64,
    pub reserved5: [u8; 24],
    pub rax: u64,
    pub star: u64,
    pub lstar: u64,
    pub cstar: u64,
    pub sf_mask: u64,
    pub kernel_gs_base: u64,
    pub sysenter_cs: u64,
    pub sysenter_esp: u64,
    pub sysenter_eip: u64,
    pub cr2: u64,
    pub reserved6: [u8; 32usize],
    pub gpat: u64,
    pub dbg_ctl: u64,
    pub br_from: u64,
    pub br_to: u64,
    pub last_excep_from: u64,
    pub last_excep_to: u64,
}
const_assert_eq!(core::mem::size_of::<SaveArea>(), 0x298);

impl SaveArea {
    // See: https://github.com/tandasat/SimpleSvm/blob/master/SimpleSvm/SimpleSvm.cpp#L893
    fn segment_access_right(segment_selector: u16, gdt_base: u64) -> u16 {
        const RPL_MASK: u16 = 3;
        let descriptor = gdt_base + (segment_selector & !RPL_MASK) as u64;

        let descriptor = descriptor as *mut u64 as *mut SegmentDescriptor;
        let descriptor = unsafe { descriptor.read_volatile() };

        let mut attribute = SegmentAttribute(0);
        attribute.set_type(descriptor.get_type() as u16);
        attribute.set_system(descriptor.get_system() as u16);
        attribute.set_dpl(descriptor.get_dpl() as u16);
        attribute.set_present(descriptor.get_present() as u16);
        attribute.set_avl(descriptor.get_avl() as u16);
        attribute.set_long_mode(descriptor.get_long_mode() as u16);
        attribute.set_default_bit(descriptor.get_default_bit() as u16);
        attribute.set_granularity(descriptor.get_granularity() as u16);

        attribute.0
    }

    // See: https://www.felixcloutier.com/x86/lsl
    fn segment_limit(selector: u16) -> u32 {
        let limit: u32;
        unsafe {
            asm!("lsl {0:e}, {1:x}", out(reg) limit, in(reg) selector, options(nostack, nomem));
        }
        limit
    }

    pub fn build(&mut self, context: Context) {
        // Like this: https://github.com/tandasat/SimpleSvm/blob/master/SimpleSvm/SimpleSvm.cpp#L1053

        // Capture the current GDT and IDT to use as initial values of the guest
        // mode.
        //
        // See:
        // - https://en.wikipedia.org/wiki/Global_Descriptor_Table
        // - https://en.wikipedia.org/wiki/Interrupt_descriptor_table
        //
        let gdt = sgdt();
        let idt = sidt();

        self.gdtr_base = gdt.base.as_u64();
        self.gdtr_limit = gdt.limit as _;

        self.idtr_base = idt.base.as_u64();
        self.idtr_limit = idt.limit as _;

        self.cs_limit = Self::segment_limit(context.seg_cs);
        self.ds_limit = Self::segment_limit(context.seg_ds);
        self.es_limit = Self::segment_limit(context.seg_es);
        self.ss_limit = Self::segment_limit(context.seg_ss);

        self.cs_selector = context.seg_cs;
        self.ds_selector = context.seg_ds;
        self.es_selector = context.seg_es;
        self.ss_selector = context.seg_ss;

        self.cs_attrib = Self::segment_access_right(context.seg_cs, gdt.base.as_u64());
        self.ds_attrib = Self::segment_access_right(context.seg_ds, gdt.base.as_u64());
        self.es_attrib = Self::segment_access_right(context.seg_es, gdt.base.as_u64());
        self.ss_attrib = Self::segment_access_right(context.seg_ss, gdt.base.as_u64());

        self.gpat = unsafe { rdmsr(IA32_PAT) };
        self.efer = unsafe { rdmsr(IA32_EFER) };
        self.cr0 = Cr0::read_raw();
        self.cr2 = unsafe { cr2() } as _;
        self.cr3 = unsafe { cr3() };
        self.cr4 = Cr4::read_raw();
        self.rflags = context.e_flags as u64;
        self.rsp = context.rsp;
        self.rip = context.rip;
    }
}
