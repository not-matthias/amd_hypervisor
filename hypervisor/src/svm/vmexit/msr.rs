use crate::svm::{
    events::EventInjection,
    utils::{guest::GuestRegs, msr::EFER_SVME},
    vcpu_data::VcpuData,
    vmexit::ExitType,
};
use x86::msr::{rdmsr, wrmsr};

pub(crate) fn handle_efer(data: &mut VcpuData, guest_regs: &mut GuestRegs) -> ExitType {
    let write_access = data.guest_vmcb.control_area.exit_info1.bits() != 0;

    if write_access {
        let low_part = guest_regs.rax as u32;
        let high_part = guest_regs.rdx as u32;
        let value = (high_part as u64) << 32 | low_part as u64;

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
        // The EFER msr, without the SVME flag.
        //
        let value = data.guest_vmcb.save_area.efer & !EFER_SVME;

        guest_regs.rax = (value as u32) as u64;
        guest_regs.rdx = (value >> 32) as u64;
    }

    ExitType::IncrementRIP
}

pub fn handle_default(data: &mut VcpuData, guest_regs: &mut GuestRegs) -> ExitType {
    let msr = guest_regs.rcx as u32;
    let write_access = data.guest_vmcb.control_area.exit_info1.bits() != 0;

    // if msr::is_valid_msr(msr) {
    //     log::warn!("Found invalid msr: {:x}", msr);
    // }

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

    ExitType::IncrementRIP
}
