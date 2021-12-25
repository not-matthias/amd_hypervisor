use nt::include::IofCompleteRequest;
use winapi::km::wdm::IO_PRIORITY::IO_NO_INCREMENT;
use winapi::km::wdm::{IoGetCurrentIrpStackLocation, DEVICE_OBJECT, IRP};
use winapi::shared::ntdef::NTSTATUS;
use winapi::shared::ntstatus::STATUS_SUCCESS;

pub const IOCTL_INSTALL: u32 = 0x22e000;
pub const IOCTL_UNLOAD: u32 = 0x22e004;
pub const IOCTL_UNUSED: u32 = 0x22e008;

#[inline(never)]
pub fn hook_handler(_: &mut DEVICE_OBJECT, irp: &mut IRP) -> NTSTATUS {
    log::info!("Hook handler called");

    let stack_location = IoGetCurrentIrpStackLocation(irp);
    let ioctl_code = unsafe { (*stack_location).Parameters.DeviceIoControl().IoControlCode };

    match ioctl_code {
        IOCTL_INSTALL => {
            log::info!("IOCTL_INSTALL");
        }
        IOCTL_UNLOAD => {
            log::info!("IOCTL_UNLOAD");
        }
        IOCTL_UNUSED => {
            log::info!("IOCTL_UNUSED");
        }
        _ => (),
    };

    unsafe { IofCompleteRequest(irp, IO_NO_INCREMENT as _) };

    STATUS_SUCCESS
}
