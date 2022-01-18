use core::arch::asm;

/// Breaks if a kernel debugger is present on the system.
pub macro dbg_break() {
    #[allow(unused_unsafe)]
    if unsafe { !*crate::utils::nt::KdDebuggerNotPresent } {
        #[allow(unused_unsafe)]
        unsafe {
            asm!("int 3")
        };
    }
}
