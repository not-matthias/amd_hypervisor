//! This module contains the definitions of functions and structures.\

#![allow(bad_style)]
#![allow(missing_docs)]

use core::mem::MaybeUninit;
use nt::include::HANDLE;
use winapi::shared::ntdef::LARGE_INTEGER;
use winapi::shared::ntdef::OBJECT_ATTRIBUTES;
use winapi::shared::ntdef::PHANDLE;
use winapi::shared::ntdef::PHYSICAL_ADDRESS;
use winapi::shared::ntdef::PVOID;
use winapi::{
    km::wdm::{KIRQL, KPROCESSOR_MODE, POOL_TYPE},
    shared::{
        basetsd::SIZE_T,
        ntdef::{NTSTATUS, PGROUP_AFFINITY, PPROCESSOR_NUMBER},
    },
    um::winnt::PCONTEXT,
};

/// `VOID KSTART_ROUTINE (_In_ PVOID StartContext);`
pub type KSTART_ROUTINE = extern "system" fn(*mut u64);

extern "system" {
    pub static KdDebuggerNotPresent: *mut bool;

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

    pub fn KeBugCheck(BugCheckCode: u32) -> !;

    pub fn ZwYieldExecution() -> NTSTATUS;

    pub fn PsCreateSystemThread(
        ThreadHandle: PHANDLE,
        DesiredAccess: u32,
        ObjectAttributes: *mut OBJECT_ATTRIBUTES,
        ProcessHandle: HANDLE,
        ClientId: *mut u64,
        StartRoutine: *const (), // *const KSTART_ROUTINE
        StartContext: *mut u64,
    ) -> NTSTATUS;

    pub fn MmGetPhysicalMemoryRanges() -> *mut PHYSICAL_MEMORY_RANGE;

    pub fn MmGetVirtualForPhysical(PhysicalAddress: PHYSICAL_ADDRESS) -> *mut u64;

    pub fn RtlCopyMemory(destination: *mut u64, source: *mut u64, length: usize);

    pub fn ExAllocatePoolWithTag(PoolType: u32, NumberOfBytes: usize, Tag: u32) -> u64;
}

// See: https://docs.microsoft.com/en-us/windows-hardware/drivers/debugger/bug-check-code-reference2#bug-check-codes
pub const MANUALLY_INITIATED_CRASH: u32 = 0x000000E2;

pub const MM_ANY_NODE_OK: u32 = 0x80000000;
pub type NODE_REQUIREMENT = u32;

#[repr(C)]
pub struct PHYSICAL_MEMORY_RANGE {
    pub base_address: PHYSICAL_ADDRESS,
    pub number_of_bytes: LARGE_INTEGER,
}

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
    pub p1_home: u64,
    pub p2_home: u64,
    pub p3_home: u64,
    pub p4_home: u64,
    pub p5: u64,
    /*
     * Previous processor mode (system services only) and previous IRQL
     * (interrupts only).
     */
    pub previous_mode: KPROCESSOR_MODE,
    pub previous_irql: KIRQL,
    /*
     * Page fault load/store indicator.
     */
    pub fault_indicator: u8,
    /*
     * Exception active indicator.
     *
     *    0 - interrupt frame.
     *    1 - exception frame.
     *    2 - service frame.
     */
    pub exception_active: u8,
    /*
     * Floating point state.
     */
    pub mx_csr: u32,
    /*
     *  Volatile registers.
     *
     * N.B. These registers are only saved on exceptions and interrupts. They
     *      are not saved for system calls.
     */
    pub rax: u64,
    pub rcx: u64,
    pub rdx: u64,
    pub r8: u64,
    pub r9: u64,
    pub r10: u64,
    pub r11: u64,
    /*
     * Gsbase is only used if the previous mode was kernel.
     *
     * GsSwap is only used if the previous mode was user.
     *
     * Note: This was originally an union (GsSwap).
     */
    pub gs_base: u64,
    /*
     * Volatile floating registers.
     *
     * N.B. These registers are only saved on exceptions and interrupts. They
     *      are not saved for system calls.
     */
    pub xmm0: u128,
    pub xmm1: u128,
    pub xmm2: u128,
    pub xmm3: u128,
    pub xmm4: u128,
    pub xmm5: u128,
    /*
     * First parameter, page fault address, context record address if user APC
     * bypass.
     *
     * Note: This was originally an union (ContextRecord).
     */
    pub fault_address: u64,
    /*
     *  Debug registers.
     */
    pub dr0: u64,
    pub dr1: u64,
    pub dr2: u64,
    pub dr3: u64,
    pub dr6: u64,
    pub dr7: u64,
    /*
     * Special debug registers.
     *
     * Note: This was originally in its own structure.
     */
    pub debug_control: u64,
    pub last_branch_to_rip: u64,
    pub last_branch_from_rip: u64,
    pub last_exception_to_rip: u64,
    pub last_exception_from_rip: u64,
    /*
     *  Segment registers
     */
    pub seg_ds: u16,
    pub seg_es: u16,
    pub seg_fs: u16,
    pub seg_gs: u16,
    /*
     * Previous trap frame address.
     */
    pub trap_frame: u64,
    /*
     * Saved nonvolatile registers RBX, RDI and RSI. These registers are only
     * saved in system service trap frames.
     */
    pub rbx: u64,
    pub rdi: u64,
    pub rsi: u64,
    /*
     * Saved nonvolatile register RBP. This register is used as a frame
     * pointer during trap processing and is saved in all trap frames.
     */
    pub rbp: u64,
    /*
     * Information pushed by hardware.
     *
     * N.B. The error code is not always pushed by hardware. For those cases
     *      where it is not pushed by hardware a dummy error code is allocated
     *      on the stack.
     *
     * Note: This was originally an union (ExceptionFrame).
     */
    pub error_code: u64,
    pub rip: u64,
    pub seg_cs: u16,
    pub fill_0: u8,
    pub logging: u8,
    pub fill_1: [u16; 2],
    pub e_flags: u32,
    pub fill_2: u32,
    pub rsp: u64,
    pub seg_ss: u16,
    pub fill_3: u16,
    pub fill_4: u32,
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
pub struct Context {
    //
    // Register parameter home addresses.
    //
    // N.B. These fields are for convenience - they could be used to extend the
    //      context record in the future.
    pub p1_home: u64,
    pub p2_home: u64,
    pub p3_home: u64,
    pub p4_home: u64,
    pub p5_home: u64,
    pub p6_home: u64,
    /*
     * Control flags.
     */
    pub context_flags: u32,
    pub mx_csr: u32,
    /*
     * Segment Registers and processor flags.
     */
    pub seg_cs: u16,
    pub seg_ds: u16,
    pub seg_es: u16,
    pub seg_fs: u16,
    pub seg_gs: u16,
    pub seg_ss: u16,
    pub e_flags: u32,
    //
    // Debug registers
    pub dr0: u64,
    pub dr1: u64,
    pub dr2: u64,
    pub dr3: u64,
    pub dr6: u64,
    pub dr7: u64,
    /*
     * Integer registers.
     */
    pub rax: u64,
    pub rcx: u64,
    pub rdx: u64,
    pub rbx: u64,
    pub rsp: u64,
    pub rbp: u64,
    pub rsi: u64,
    pub rdi: u64,
    pub r8: u64,
    pub r9: u64,
    pub r10: u64,
    pub r11: u64,
    pub r12: u64,
    pub r13: u64,
    pub r14: u64,
    pub r15: u64,
    /*
     * Program counter.
     */
    pub rip: u64,
    /*
     * Floating point state.
     */
    pub flt_save: XMM_SAVE_AREA,
    /*
     * Vector registers.
     */
    pub vector_register: [u128; 26],
    pub vector_control: u64,
    /*
     * Special debug control registers.
     */
    pub debug_control: u64,
    pub last_branch_to_rip: u64,
    pub last_branch_from_rip: u64,
    pub last_exception_to_rip: u64,
    pub last_exception_from_rip: u64,
}

impl Context {
    pub fn capture() -> Self {
        let mut context: MaybeUninit<Context> = MaybeUninit::uninit();

        unsafe { RtlCaptureContext(context.as_mut_ptr() as _) };

        unsafe { context.assume_init() }
    }
}

pub macro assert_paged_code() {
    #[cfg(not(feature = "no-assertions"))]
    assert!(
        unsafe { $crate::nt::irql::KeGetCurrentIrql() } <= $crate::nt::irql::APC_LEVEL,
        "Called at IRQL > APC_LEVEL",
    );
}
