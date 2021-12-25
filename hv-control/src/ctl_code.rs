//! Helper functions for working with ctl codes.

#![allow(missing_docs)]

use crate::ctl_code;

pub const IOCTL_INSTALL: u32 = ctl_code!(0x800);
pub const IOCTL_UNLOAD: u32 = ctl_code!(0x801);
pub const IOCTL_UNUSED: u32 = ctl_code!(0x802);

pub const METHOD_BUFFERED: u32 = 0;
pub const METHOD_IN_DIRECT: u32 = 1;
pub const METHOD_OUT_DIRECT: u32 = 2;
pub const METHOD_NEITHER: u32 = 3;

pub const FILE_ANY_ACCESS: u32 = 0;
pub const FILE_SPECIAL_ACCESS: u32 = FILE_ANY_ACCESS;
pub const FILE_READ_ACCESS: u32 = 0x0001;
pub const FILE_WRITE_ACCESS: u32 = 0x0002;

pub const FILE_DEVICE_UNKNOWN: u32 = 0x00000022;

/// Creates a new ctl code from the parameters.
pub const fn ctl_code_fn(device_type: u32, function: u32, method: u32, access: u32) -> u32 {
    device_type << 16 | access << 14 | function << 2 | method
}

/// Creates a new ioctl code for the specified value.
#[macro_export]
macro_rules! ctl_code {
    ($x:expr) => {
        $crate::ctl_code::ctl_code_fn(
            $crate::ctl_code::FILE_DEVICE_UNKNOWN,
            $x,
            $crate::ctl_code::METHOD_BUFFERED,
            $crate::ctl_code::FILE_READ_ACCESS | $crate::ctl_code::FILE_WRITE_ACCESS,
        )
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ctl_code() {
        assert_eq!(ctl_code!(0x800), 0x22e000);
        assert_eq!(ctl_code!(0x801), 0x22e004);
        assert_eq!(ctl_code!(0x802), 0x22e008);
    }
}