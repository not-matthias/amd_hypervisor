//! See `Appendix B - Layout of VMCB` in AMD64 Architecture Programmer’s Manual Volume 2: System Programming.

use crate::svm::vmcb::control_area::ControlArea;
use crate::svm::vmcb::save_area::SaveArea;

pub mod control_area;
pub mod save_area;

const VMCB_RESERVED_SIZE: usize =
    0x1000 - core::mem::size_of::<ControlArea>() - core::mem::size_of::<SaveArea>();

/// # Layout
///
/// The VMCB is divided into two areas—the first one contains various control bits including the
/// intercept vectors and the second one contains saved guest state.
#[repr(C)]
pub struct Vmcb {
    /// Describes the layout of the control area of the VMCB, which starts at offset zero within the
    /// VMCB page. The control area is padded to a size of 1024 bytes. All unused bytes must be zero, as they
    /// are reserved for future expansion. It is recommended that software zero out any newly allocated
    /// VMCB.
    pub control_area: ControlArea,

    /// Describes the fields within the state-save area; note that the table lists offsets
    /// relative to the state-save area (not the VMCB as a whole).
    pub save_area: SaveArea,

    pub reserved: [u8; VMCB_RESERVED_SIZE],
}
