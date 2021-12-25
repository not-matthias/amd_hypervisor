//! This module contains the definitions of functions and structures.\

#![allow(bad_style)]
#![allow(missing_docs)]

use winapi::shared::ntdef::PVOID;
use winapi::{
    km::wdm::{KIRQL, KPROCESSOR_MODE, POOL_TYPE},
    shared::{
        basetsd::SIZE_T,
        ntdef::{NTSTATUS, PGROUP_AFFINITY, PHYSICAL_ADDRESS, PPROCESSOR_NUMBER},
    },
    um::winnt::PCONTEXT,
};

extern "system" {
    pub fn _sgdt(Descriptor: PVOID);

    pub fn ExAllocatePool(PoolType: POOL_TYPE, NumberOfBytes: SIZE_T) -> PVOID;

    pub fn ExFreePool(P: PVOID);

    pub fn memset(Dst: PVOID, Val: u64, Size: usize) -> PVOID;

    pub fn RtlInitializeBitMap(
        BitMapHeader: PRTL_BITMAP,
        BitMapBuffer: *mut u32,
        SizeOfBitMap: u32,
    );

    pub fn RtlClearAllBits(BitMapHeader: PRTL_BITMAP);

    pub fn RtlSetBits(BitMapHeader: PRTL_BITMAP, StartingIndex: u32, NumberToSet: u32);

    pub fn KeQueryActiveProcessorCountEx(GroupNumber: u16) -> u32;

    pub fn KeGetProcessorNumberFromIndex(ProcIndex: u32, ProcNumber: PPROCESSOR_NUMBER)
        -> NTSTATUS;

    pub fn KeSetSystemGroupAffinityThread(
        Affinity: PGROUP_AFFINITY,
        PreviousAffinity: PGROUP_AFFINITY,
    );

    pub fn KeRevertToUserGroupAffinityThread(PreviousAffinity: PGROUP_AFFINITY);

    pub fn RtlCaptureContext(ContextRecord: PCONTEXT);

    pub fn MmGetPhysicalAddress(BaseAddress: PVOID) -> PHYSICAL_ADDRESS;

    pub fn MmAllocateContiguousMemorySpecifyCacheNode(
        NumberOfBytes: SIZE_T,
        LowestAcceptableAddress: PHYSICAL_ADDRESS,
        HighestAcceptableAddress: PHYSICAL_ADDRESS,
        BoundaryAddressMultiple: PHYSICAL_ADDRESS,
        CacheType: MEMORY_CACHING_TYPE,
        PreferredNode: NODE_REQUIREMENT,
    ) -> PVOID;

    pub fn MmFreeContiguousMemory(BaseAddress: PVOID);

}

pub const MM_ANY_NODE_OK: u32 = 0x80000000;
pub type NODE_REQUIREMENT = u32;

#[repr(C)]
pub struct RTL_BITMAP {
    SizeOfBitMap: u32,
    Buffer: *mut u32,
}
pub type PRTL_BITMAP = *mut RTL_BITMAP;

/// Size is 0x190 (400)
#[repr(C)]
pub struct KTRAP_FRAME {
    /*
     * Home address for the parameter registers.
     */
    p1_home: u64,
    p2_home: u64,
    p3_home: u64,
    p4_home: u64,
    p5: u64,
    /*
     * Previous processor mode (system services only) and previous IRQL
     * (interrupts only).
     */
    previous_mode: KPROCESSOR_MODE,
    previous_irql: KIRQL,
    /*
     * Page fault load/store indicator.
     */
    fault_indicator: u8,
    /*
     * Exception active indicator.
     *
     *    0 - interrupt frame.
     *    1 - exception frame.
     *    2 - service frame.
     */
    exception_active: u8,
    /*
     * Floating point state.
     */
    mx_csr: u32,
    /*
     *  Volatile registers.
     *
     * N.B. These registers are only saved on exceptions and interrupts. They
     *      are not saved for system calls.
     */
    rax: u64,
    rcx: u64,
    rdx: u64,
    r8: u64,
    r9: u64,
    r10: u64,
    r11: u64,
    /*
     * Gsbase is only used if the previous mode was kernel.
     *
     * GsSwap is only used if the previous mode was user.
     *
     * Note: This was originally an union (GsSwap).
     */
    gs_base: u64,
    /*
     * Volatile floating registers.
     *
     * N.B. These registers are only saved on exceptions and interrupts. They
     *      are not saved for system calls.
     */
    xmm0: u128,
    xmm1: u128,
    xmm2: u128,
    xmm3: u128,
    xmm4: u128,
    xmm5: u128,
    /*
     * First parameter, page fault address, context record address if user APC
     * bypass.
     *
     * Note: This was originally an union (ContextRecord).
     */
    fault_address: u64,
    /*
     *  Debug registers.
     */
    dr0: u64,
    dr1: u64,
    dr2: u64,
    dr3: u64,
    dr6: u64,
    dr7: u64,
    /*
     * Special debug registers.
     *
     * Note: This was originally in its own structure.
     */
    debug_control: u64,
    last_branch_to_rip: u64,
    last_branch_from_rip: u64,
    last_exception_to_rip: u64,
    last_exception_from_rip: u64,
    /*
     *  Segment registers
     */
    seg_ds: u16,
    seg_es: u16,
    seg_fs: u16,
    seg_gs: u16,
    /*
     * Previous trap frame address.
     */
    trap_frame: u64,
    /*
     * Saved nonvolatile registers RBX, RDI and RSI. These registers are only
     * saved in system service trap frames.
     */
    rbx: u64,
    rdi: u64,
    rsi: u64,
    /*
     * Saved nonvolatile register RBP. This register is used as a frame
     * pointer during trap processing and is saved in all trap frames.
     */
    rbp: u64,
    /*
     * Information pushed by hardware.
     *
     * N.B. The error code is not always pushed by hardware. For those cases
     *      where it is not pushed by hardware a dummy error code is allocated
     *      on the stack.
     *
     * Note: This was originally an union (ExceptionFrame).
     */
    error_code: u64,
    rip: u64,
    seg_cs: u16,
    fill_0: u8,
    logging: u8,
    fill_1: [u16; 2],
    e_flags: u32,
    fill_2: u32,
    rsp: u64,
    seg_ss: u16,
    fill_3: u16,
    fill_4: u32,
}

#[repr(C)]
pub enum MEMORY_CACHING_TYPE {
    MmNonCached = 0,
    MmCached = 1,
    MmWriteCombined = 2,
    MmHardwareCoherentCached,
    MmNonCachedUnordered,
    MmUSWCCached,
    MmMaximumCacheType,
    MmNotMapped = -1,
}

#[repr(C, align(16))]
pub struct M128A {
    low: u64,
    high: u64,
}

#[repr(C, align(16))]
pub struct XSAVE_FORMAT {
    control_word: u16,
    status_word: u16,
    tag_word: u8,
    reserved_1: u8,
    error_opcode: u16,
    error_offset: u32,
    error_selector: u16,
    reserved_2: u16,
    data_offset: u32,
    data_selector: u16,
    reserved_3: u16,
    mx_csr: u32,
    mx_csr_mask: u32,
    float_registers: [u128; 8],
    #[cfg(target_pointer_width = "64")]
    xmm_registers: [u128; 16],
    #[cfg(target_pointer_width = "32")]
    xmm_registers: [u128; 8],
    #[cfg(target_pointer_width = "64")]
    reserved_4: [u8; 96],
    #[cfg(target_pointer_width = "32")]
    reserved_4: [u8; 224],
}
pub type XMM_SAVE_AREA = XSAVE_FORMAT;

///
/// Context Frame
///
///  This frame has a several purposes: 1) it is used as an argument to
///  NtContinue, 2) it is used to constuct a call frame for APC delivery,
///  and 3) it is used in the user level thread creation routines.
///
///
/// The flags field within this record controls the contents of a CONTEXT
/// record.
///
/// If the context record is used as an input parameter, then for each
/// portion of the context record controlled by a flag whose value is
/// set, it is assumed that that portion of the context record contains
/// valid context. If the context record is being used to modify a threads
/// context, then only that portion of the threads context is modified.
///
/// If the context record is used as an output parameter to capture the
/// context of a thread, then only those portions of the thread's context
/// corresponding to set flags will be returned.
///
/// CONTEXT_CONTROL specifies SegSs, Rsp, SegCs, Rip, and EFlags.
///
/// CONTEXT_INTEGER specifies Rax, Rcx, Rdx, Rbx, Rbp, Rsi, Rdi, and R8-R15.
///
/// CONTEXT_SEGMENTS specifies SegDs, SegEs, SegFs, and SegGs.
///
/// CONTEXT_FLOATING_POINT specifies Xmm0-Xmm15.
///
/// CONTEXT_DEBUG_REGISTERS specifies Dr0-Dr3 and Dr6-Dr7.
///
/// Size: 1232 bytes (confirmed)
#[repr(C, align(16))]
pub struct CONTEXT {
    //
    // Register parameter home addresses.
    //
    // N.B. These fields are for convience - they could be used to extend the
    //      context record in the future.
    p1_home: u64,
    p2_home: u64,
    p3_home: u64,
    p4_home: u64,
    p5_home: u64,
    p6_home: u64,
    /*
     * Control flags.
     */
    context_flags: u32,
    mx_csr: u32,
    /*
     * Segment Registers and processor flags.
     */
    seg_cs: u16,
    seg_ds: u16,
    seg_es: u16,
    seg_fs: u16,
    seg_gs: u16,
    seg_ss: u16,
    e_flags: u32,
    //
    // Debug registers
    dr0: u64,
    dr1: u64,
    dr2: u64,
    dr3: u64,
    dr6: u64,
    dr7: u64,
    /*
     * Integer registers.
     */
    rax: u64,
    rcx: u64,
    rdx: u64,
    rbx: u64,
    rsp: u64,
    rbp: u64,
    rsi: u64,
    rdi: u64,
    r8: u64,
    r9: u64,
    r10: u64,
    r11: u64,
    r12: u64,
    r13: u64,
    r14: u64,
    r15: u64,
    /*
     * Program counter.
     */
    rip: u64,
    /*
     * Floating point state.
     */
    flt_save: XMM_SAVE_AREA,
    /*
     * Vector registers.
     */
    vector_register: [u128; 26],
    vector_control: u64,
    /*
     * Special debug control registers.
     */
    debug_control: u64,
    last_branch_to_rip: u64,
    last_branch_from_rip: u64,
    last_exception_to_rip: u64,
    last_exception_from_rip: u64,
}
