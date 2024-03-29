use winapi::km::wdm::KIRQL;

#[link(name = "ntoskrnl")]
extern "system" {
    pub fn KeGetCurrentIrql() -> KIRQL;

    pub fn KeRaiseIrqlToDpcLevel() -> KIRQL;

    pub fn KfRaiseIrql(new_irql: KIRQL) -> KIRQL;

    pub fn KeLowerIrql(new_irql: KIRQL);
}

pub unsafe fn KeRaiseIrql(new_irql: KIRQL, old_irql: *mut KIRQL) {
    *old_irql = KfRaiseIrql(new_irql);
}

/// Passive release level
pub const PASSIVE_LEVEL: KIRQL = 0;
/// Lowest interrupt level
pub const LOW_LEVEL: KIRQL = 0;
/// APC interrupt level
pub const APC_LEVEL: KIRQL = 1;
/// Dispatcher level
pub const DISPATCH_LEVEL: KIRQL = 2;
/// CMCI interrupt level
pub const CMCI_LEVEL: KIRQL = 5;

/// Interval clock level
pub const CLOCK_LEVEL: KIRQL = 13;
/// Interprocessor interrupt level
pub const IPI_LEVEL: KIRQL = 14;
/// Deferred Recovery Service level
pub const DRS_LEVEL: KIRQL = 14;
/// Power failure level
pub const POWER_LEVEL: KIRQL = 14;
/// Timer used for profiling.
pub const PROFILING_LEVEL: KIRQL = 15;
/// Highest interrupt level
pub const HIGH_LEVEL: KIRQL = 15;

pub macro assert_paged_code() {
    assert!(
        unsafe { $crate::utils::nt::irql::KeGetCurrentIrql() }
            <= $crate::utils::nt::irql::APC_LEVEL,
        "Called at IRQL > APC_LEVEL",
    );
}
