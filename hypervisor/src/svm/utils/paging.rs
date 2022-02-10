use x86::bits64::paging::{PDFlags, PDPTFlags, PML4Flags, PTFlags, MAXPHYADDR};

pub const _512GB: u64 = 512 * 1024 * 1024 * 1024;
pub const _1GB: u64 = 1024 * 1024 * 1024;
pub const _2MB: usize = 2 * 1024 * 1024;
pub const _4KB: usize = 4 * 1024;

pub const PAGE_SHIFT: u64 = 12;
pub const PFN_MASK: u64 = ((1 << MAXPHYADDR) - 1) & !0xfff;

const RW: u64 = 0b1;

/// The NX bit can only be set when the no-execute page-protection feature is
/// enabled by setting EFER.NXE to 1 (see “Extended Feature Enable Register
/// (EFER)” on page 56). If EFER.NXE=0, the NX bit is treated as reserved. In
/// this case, a page-fault exception (#PF) occurs if the NX bit is not
/// cleared to 0.
const NX: u64 = 0b1 << 63;

#[derive(Debug, Ord, PartialOrd, Eq, PartialEq, Copy, Clone)]
pub enum AccessType {
    ReadWrite,
    ReadWriteExecute,
}

impl AccessType {
    pub(crate) fn pml4_flags(self) -> PML4Flags {
        match self {
            AccessType::ReadWrite => PML4Flags::P | PML4Flags::RW | PML4Flags::US | PML4Flags::XD,
            AccessType::ReadWriteExecute => PML4Flags::P | PML4Flags::RW | PML4Flags::US,
        }
    }

    pub(crate) fn pdpt_flags(self) -> PDPTFlags {
        match self {
            AccessType::ReadWrite => PDPTFlags::P | PDPTFlags::RW | PDPTFlags::US | PDPTFlags::XD,
            AccessType::ReadWriteExecute => PDPTFlags::P | PDPTFlags::RW | PDPTFlags::US,
        }
    }

    pub(crate) fn pd_flags(self) -> PDFlags {
        match self {
            AccessType::ReadWrite => PDFlags::P | PDFlags::RW | PDFlags::US | PDFlags::XD,
            AccessType::ReadWriteExecute => PDFlags::P | PDFlags::RW | PDFlags::US,
        }
    }

    pub(crate) fn pt_flags(self) -> PTFlags {
        match self {
            AccessType::ReadWrite => {
                PTFlags::from_iter([PTFlags::P, PTFlags::RW, PTFlags::US, PTFlags::XD])
            }
            AccessType::ReadWriteExecute => {
                PTFlags::from_iter([PTFlags::P, PTFlags::RW, PTFlags::US])
            }
        }
    }

    pub(crate) fn modify_2mb(&self, mut flags: PDFlags) -> PDFlags {
        match self {
            AccessType::ReadWrite => {
                flags.insert(PDFlags::RW);
                flags.insert(PDFlags::XD);
            }
            AccessType::ReadWriteExecute => {
                flags.insert(PDFlags::RW);
                flags.remove(PDFlags::XD);
            }
        }

        flags
    }

    pub(crate) fn modify_4kb(&self, mut flags: PTFlags) -> PTFlags {
        match self {
            AccessType::ReadWrite => {
                flags.insert(PTFlags::RW);
                flags.insert(PTFlags::XD);
            }
            AccessType::ReadWriteExecute => {
                flags.insert(PTFlags::RW);
                flags.remove(PTFlags::XD);
            }
        }

        flags
    }
}

/// Calculates how many pages are required to hold the specified number of
/// bytes.
pub macro bytes_to_pages($bytes:expr) {
    ($bytes >> crate::svm::utils::paging::PAGE_SHIFT) as usize
}
