use crate::svm::{
    data::{guest::GuestRegisters, msr_bitmap::EFER_SVME, processor_data::ProcessorData},
    events::EventInjection,
    vmexit::ExitType,
};
use x86::msr::{rdmsr, wrmsr, IA32_EFER};

pub fn handle_msr(data: &mut ProcessorData, guest_regs: &mut GuestRegisters) -> ExitType {
    let msr = guest_regs.rcx as u32;
    let write_access = data.guest_vmcb.control_area.exit_info1.bits() != 0;

    // TODO: Hide psave

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
