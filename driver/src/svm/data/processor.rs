use crate::nt::memory::PAGE_SIZE;
use crate::svm::data::shared_data::SharedData;
use crate::{nt::include::KTRAP_FRAME, svm::vmcb::Vmcb};
use nt::include::PVOID;

pub const KERNEL_STACK_SIZE: usize = 0x6000;
pub const STACK_CONTENTS_SIZE: usize =
    KERNEL_STACK_SIZE - (core::mem::size_of::<PVOID>() * 6) - core::mem::size_of::<KTRAP_FRAME>();

#[repr(C)]
pub struct HostStackLayout {
    stack_contents: [u8; STACK_CONTENTS_SIZE],
    trap_frame: KTRAP_FRAME,

    /// HostRsp
    guest_vmcb_pa: u64,
    host_vmcb_pa: u64,

    // TODO: Can we somehow circumvent these pointer?
    self_data: *mut ProcessorData,
    shared_data: *mut SharedData,

    /// To keep HostRsp 16 bytes aligned
    padding_1: u64,
    reserved_1: u64,
}

/// The data for a single **virtual** processor.
#[repr(C, align(4096))]
pub struct ProcessorData {
    /// Taken from SimpleSvm.
    ///
    /// ```
    ///  Low     HostStackLimit[0]                        StackLimit
    ///  ^       ...
    ///  ^       HostStackLimit[KERNEL_STACK_SIZE - 2]    StackBase
    ///  High    HostStackLimit[KERNEL_STACK_SIZE - 1]    StackBase
    /// ```
    ///
    /// Can be transmuted to `HostStackLayout`.
    host_stack_limit: [u8; KERNEL_STACK_SIZE],
    guest_vmcb: Vmcb,
    host_vmcb: Vmcb,
    host_state_area: [u8; PAGE_SIZE],
}
