use crate::nt::include::KdDebuggerNotPresent;
use core::arch::asm;

/// Breaks if a kernel debugger is present on the system.
#[inline(always)]
pub fn dbg_break() {
    if unsafe { !*KdDebuggerNotPresent } {
        unsafe { asm!("int 3") };
    }
}

pub macro dbg_break() {
    #[allow(unused_unsafe)]
    if unsafe { !*crate::nt::include::KdDebuggerNotPresent } {
        #[allow(unused_unsafe)]
        unsafe {
            asm!("int 3")
        };
    }
}
