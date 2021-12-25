//! Checks whether the current system is able to run the hypervisor.

use x86::cpuid::CpuId;
use x86::msr::rdmsr;

/// Checks whether svm is supported by the processor.
///
/// # Recommended Algorithm
/// This algorithm has been taken from section `15.4 Enabling SVM` from the AMD
/// manual.
/// ```pseudocode
/// if (CPUID Fn8000_0001_ECX[SVM] == 0)
///     return SVM_NOT_AVAIL;
///
/// if (VM_CR.SVMDIS == 0)
///     return SVM_ALLOWED;
///
/// if (CPUID Fn8000_000A_EDX[SVML]==0)
///     return SVM_DISABLED_AT_BIOS_NOT_UNLOCKABLE
///     // the user must change a platform firmware setting to enable SVM
/// else
///     return SVM_DISABLED_WITH_KEY;
///     // SVMLock may be unlockable; consult platform firmware or TPM to obtain the key.
/// ```
pub fn is_svm_supported() -> bool {
    // Check `CPUID Fn8000_0001_ECX[SVM] == 0`
    //
    let Some(result) = CpuId::new().get_extended_processor_and_feature_identifiers() else { return false };
    if !result.has_svm() {
        log::warn!("Processor does not support SVM");
        return false;
    }

    // Check `VM_CR.SVMDIS == 0`
    //
    // See in the AMD Manual '15.30.1  VM_CR MSR (C001_0114h)'
    //
    const SVM_MSR_VM_CR: u32 = 0xC001_0114;
    const SVM_VM_CR_SVMDIS: u64 = 1 << 4;

    let vm_cr = unsafe { rdmsr(SVM_MSR_VM_CR) };
    if (vm_cr & SVM_VM_CR_SVMDIS) == 0 {
        return true;
    }

    // Check `CPUID Fn8000_000A_EDX[SVML]==0`
    //
    if CpuId::new()
        .get_svm_info()
        .map(|svm_info| svm_info.has_svm_lock())
        .unwrap_or_default()
    {
        log::warn!("SVM is locked at BIOS level. You must change a platform firmware setting to enable SVM.");
    } else {
        log::warn!(
            "SVMLock may be unlockable; consult platform firmware or TPM to obtain the key."
        );
    }

    false
}
