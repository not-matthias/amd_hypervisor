use ntapi::{
    ntioapi::{NtOpenFile, FILE_NON_DIRECTORY_FILE},
    ntrtl::RtlInitUnicodeString,
};
use widestring::U16CString;
use winapi::um::ioapiset::DeviceIoControl;
use winapi::{
    shared::ntdef::{InitializeObjectAttributes, OBJECT_ATTRIBUTES, OBJ_CASE_INSENSITIVE},
    um::winnt::{FILE_GENERIC_READ, FILE_GENERIC_WRITE, FILE_SHARE_READ, FILE_SHARE_WRITE},
};
use winapi::um::handleapi::CloseHandle;
use crate::ctl_code::{IOCTL_INSTALL, IOCTL_UNLOAD, IOCTL_UNUSED};

/// Creates a new ctl code from the parameters.
pub const fn ctl_code(device_type: u32, function: u32, method: u32, access: u32) -> u32 {
    device_type << 16 | access << 14 | function << 2 | method
}

pub mod ctl_code;

#[derive(Debug)]
pub struct IoctlConnector {
    pid: u32,
    handle: i32,
}

impl IoctlConnector {
    pub fn new(driver: U16CString, pid: u32) -> Option<Self> {
        let handle = Self::connect(driver)?;

        Some(Self { pid, handle })
    }

    pub fn connect(driver: U16CString) -> Option<i32> {
        let mut path = std::mem::MaybeUninit::uninit();
        unsafe { RtlInitUnicodeString(path.as_mut_ptr(), driver.as_ptr()) };

        // Initialize the driver name
        //
        let mut object_attributes = std::mem::MaybeUninit::<OBJECT_ATTRIBUTES>::uninit();
        unsafe {
            InitializeObjectAttributes(
                object_attributes.as_mut_ptr(),
                path.as_mut_ptr(),
                OBJ_CASE_INSENSITIVE,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
            )
        };

        // Open the file
        //
        let mut handle = std::mem::MaybeUninit::uninit();
        let mut io_status_block = std::mem::MaybeUninit::uninit();
        let status = unsafe {
            NtOpenFile(
                handle.as_mut_ptr(),
                FILE_GENERIC_READ | FILE_GENERIC_WRITE,
                object_attributes.as_mut_ptr(),
                io_status_block.as_mut_ptr(),
                FILE_SHARE_READ | FILE_SHARE_WRITE,
                FILE_NON_DIRECTORY_FILE,
            )
        };

        match status {
            0x0000_0000 /* STATUS_SUCCESS */ => {
                Some(unsafe { handle.assume_init() } as i32)
            }
            _ => None,
        }
    }

    pub fn call<T>(&self, ioctl: u32, parameter: *mut T) -> Option<()> {
        let status = unsafe {
            DeviceIoControl(
                self.handle as *mut _,
                ioctl,
                std::ptr::null_mut(),
                0,
                std::ptr::null_mut(),
                0,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
            )
        };

        // Return the status
        //
        println!("Status: {:?}", status);
        if status == 0 {
            None
        } else {
            Some(())
        }
    }
}

impl Drop for IoctlConnector {
    fn drop(&mut self) {
        unsafe { CloseHandle(self.handle as *mut _) };
    }
}

fn main() {
    let result = IoctlConnector::new(U16CString::from_str("\\Device\\Null").unwrap(), 0).unwrap();
    println!("{:?}", result);

    result.call(IOCTL_INSTALL, 42 as *mut u32).expect("Failed to send ioctl");
    // result.call(IOCTL_UNLOAD, 42 as *mut u32).expect("Failed to send ioctl");
}
