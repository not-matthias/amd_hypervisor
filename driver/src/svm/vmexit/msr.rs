use crate::svm::{
    data::{guest::GuestRegisters, msr_bitmap::EFER_SVME, processor_data::ProcessorData},
    events::EventInjection,
    vmexit::ExitType,
};
use x86::msr::{rdmsr, wrmsr, IA32_EFER};

/// Checks whether the specified MSR is in the [Open-Source Register Reference for AMD CPUs](https://developer.amd.com/wp-content/resources/56255_3_03.PDF). See Page 260, `Memory Map - MSR`.
///
/// See also: `MSR Cross-Reference` in the AMD64 Architecture Programmer’s
/// Manual Volume 2:System Programming. TODO: Extract and compare from there
fn is_valid_msr(msr: u32) -> bool {
    // MSRs - MSR0000_xxxx
    (0x0000_0000..=0x0000_0001).contains(&msr)
        || (0x0000_0010..=0x0000_02FF).contains(&msr)
        || (0x0000_0400..=0x0000_0403).contains(&msr)
        || (0x0000_0404..=0x0000_0407).contains(&msr)
        || (0x0000_0408..=0x0000_040B).contains(&msr)
        || (0x0000_040C..=0x0000_040F).contains(&msr)
        || (0x0000_0414..=0x0000_0417).contains(&msr)
        || (0x0000_0418..=0x0000_041B).contains(&msr)
        || (0x0000_041C..=0x0000_043B).contains(&msr)
        || (0x0000_043C..=0x0000_0443).contains(&msr)
        || (0x0000_044C..=0x0000_044F).contains(&msr)
        || (0x0000_0450..=0x0000_0457).contains(&msr)
        || (0x0000_0458..=0x0000_045B).contains(&msr)
        // MSRs - MSRC000_0xxx
        || (0xC000_0080..=0xC000_0410).contains(&msr)
        || (0xC000_2000..=0xC000_2009).contains(&msr)
        || (0xC000_2010..=0xC000_2016).contains(&msr)
        || (0xC000_2020..=0xC000_2029).contains(&msr)
        || (0xC000_2030..=0xC000_2036).contains(&msr)
        || (0xC000_2040..=0xC000_2049).contains(&msr)
        || (0xC000_2050..=0xC000_2056).contains(&msr)
        || (0xC000_2060..=0xC000_2066).contains(&msr)
        || (0xC000_2070..=0xC000_20E9).contains(&msr)
        || (0xC000_20F0..=0xC000_210A).contains(&msr)
        || (0xC000_2130..=0xC000_2136).contains(&msr)
        || (0xC000_2140..=0xC000_2159).contains(&msr)
        || (0xC000_2160..=0xC000_2169).contains(&msr)
        // MSRs - MSRC001_0xxx
        || (0xC001_0000..=0xC001_029B).contains(&msr)
        || (0xC001_0400..=0xC001_0406).contains(&msr)
        || (0xC001_0407..=0xC001_040E).contains(&msr)
        || (0xC001_0413..=0xC001_0416).contains(&msr)
        // MSRs - MSRC001_1xxx
        || (0xC001_1002..=0xC001_103C).contains(&msr)
}

pub fn handle_msr(data: &mut ProcessorData, guest_regs: &mut GuestRegisters) -> ExitType {
    let msr = guest_regs.rcx as u32;
    let write_access = data.guest_vmcb.control_area.exit_info1.bits() != 0;

    // TODO: Hide psave

    // TODO: Is this needed? Since we only get msr from inside the msr permission
    // bitmap?
    if !is_valid_msr(msr) {
        EventInjection::gp().inject(data);
        return ExitType::IncrementRIP;
    }

    // Prevent IA32_EFER from being modified
    //
    if msr == IA32_EFER {
        #[cfg(not(feature = "no-assertions"))]
        assert!(write_access);

        let low_part = guest_regs.rax as u32;
        let high_part = guest_regs.rdx as u32;
        let value = (high_part as u64) << 32 | low_part as u64;

        // TODO: Hide: EFER_SVME

        // The guest is trying to enable SVM.
        //
        // Inject a #GP exception if the guest attempts to clear the flag. The
        // protection of this bit is required because clearing it would result
        // in undefined behavior.
        //
        if value & EFER_SVME != 0 {
            EventInjection::gp().inject(data);
        }

        // Otherwise, update the msr as requested.
        //
        // Note: The value should be checked beforehand to not allow any illegal values
        // and inject a #GP as needed. If that is not done, the hypervisor attempts to
        // resume the guest with an invalid EFER value and immediately receives
        // #VMEXIT due to VMEXIT_INVALID. This would in this case, result in a
        // bug check.
        //
        // See `Extended Feature Enable Register (EFER)` for what values are allowed.
        // TODO: Implement this check
        //
        data.guest_vmcb.save_area.efer = value;
    } else {
        // Execute rdmsr or wrmsr as requested by the guest.
        //
        // Important: This can bug check if the guest tries to access an MSR that is not
        // supported by            the host. See SimpleSvm for more information
        // on how to handle this correctly.
        //
        if write_access {
            let low_part = guest_regs.rax as u32;
            let high_part = guest_regs.rdx as u32;

            let value = (high_part as u64) << 32 | low_part as u64;

            unsafe { wrmsr(msr, value) };
        } else {
            let value = unsafe { rdmsr(msr) };

            guest_regs.rax = (value as u32) as u64;
            guest_regs.rdx = (value >> 32) as u64;
        }
    }

    ExitType::IncrementRIP
}
