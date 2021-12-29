use crate::debug::dbg_break;
use crate::nt::addresses::physical_address;
use crate::nt::include::{KeBugCheck, MANUALLY_INITIATED_CRASH};
use crate::svm::data::guest::{GuestContext, GuestRegisters};
use crate::svm::data::msr_bitmap::EFER_SVME;
use crate::svm::data::processor::ProcessorData;
use crate::svm::events::EventInjection;
use crate::svm::vmcb::control_area::VmExitCode;
use core::arch::asm;
use x86::cpuid::cpuid;
use x86::msr::{rdmsr, wrmsr, IA32_EFER};

pub const CPUID_DEVIRTUALIZE: u64 = 0x41414141;

pub fn handle_cpuid(_data: *mut ProcessorData, guest_context: &mut GuestContext) {
    // Execute cpuid as requested
    //
    let leaf = unsafe { (*guest_context.guest_regs).rax };
    let subleaf = unsafe { (*guest_context.guest_regs).rcx };

    let mut cpuid = cpuid!(leaf, subleaf);

    // Modify certain leafs
    //
    const CPUID_PROCESSOR_AND_PROCESSOR_FEATURE_IDENTIFIERS: u64 = 0x00000001;
    const CPUID_FN0000_0001_ECX_HYPERVISOR_PRESENT: u32 = 1 << 31;

    const CPUID_HV_VENDOR_AND_MAX_FUNCTIONS: u64 = 0x40000000;
    const CPUID_HV_MAX: u32 = CPUID_HV_INTERFACE as u32;

    const CPUID_HV_INTERFACE: u64 = 0x40000001;

    match leaf {
        CPUID_PROCESSOR_AND_PROCESSOR_FEATURE_IDENTIFIERS => {
            // Indicate presence of a hypervisor by setting the bit that are
            // reserved for use by hypervisor to indicate guest status. See "CPUID
            // Fn0000_0001_ECX Feature Identifiers".
            //
            cpuid.ecx |= CPUID_FN0000_0001_ECX_HYPERVISOR_PRESENT;
        }
        CPUID_HV_VENDOR_AND_MAX_FUNCTIONS => {
            cpuid.eax = CPUID_HV_MAX;
            cpuid.ebx = 0x42;
            cpuid.ecx = 0x42;
            cpuid.edx = 0x42;
        }
        CPUID_HV_INTERFACE => {
            // Return non Hv#1 value. This indicate that the SimpleSvm does NOT
            // conform to the Microsoft hypervisor interface.
            //
            cpuid.eax = u32::from_le_bytes(*b"0#vH"); // Hv#0
            cpuid.ebx = 0;
            cpuid.ecx = 0;
            cpuid.edx = 0;
        }
        CPUID_DEVIRTUALIZE => {
            guest_context.exit_vm = true;
        }
        _ => {}
    }

    // Store the result
    //
    unsafe {
        (*guest_context.guest_regs).rax = cpuid.eax as u64;
        (*guest_context.guest_regs).rbx = cpuid.ebx as u64;
        (*guest_context.guest_regs).rcx = cpuid.ecx as u64;
        (*guest_context.guest_regs).rdx = cpuid.edx as u64;
    }
}

pub fn handle_msr(data: *mut ProcessorData, guest_context: &mut GuestContext) {
    let msr = unsafe { (*guest_context.guest_regs).rcx as u32 };
    let write_access = unsafe { (*data).guest_vmcb.control_area.exit_info1 } != 0;

    // Prevent IA32_EFER from being modified
    //
    if msr == IA32_EFER {
        assert!(write_access);

        let low_part = unsafe { (*guest_context.guest_regs).rax as u32 };
        let high_part = unsafe { (*guest_context.guest_regs).rdx as u32 };
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
        // and inject a #GP as needed. If that is not done, the hypervisor attempts to resume
        // the guest with an invalid EFER value and immediately receives #VMEXIT due to VMEXIT_INVALID.
        // This would in this case, result in a bug check.
        //
        // See `Extended Feature Enable Register (EFER)` for what values are allowed.
        // TODO: Implement this check
        //
        unsafe { (*data).guest_vmcb.save_area.efer = value };
    } else {
        // Execute rdmsr or wrmsr as requested by the guest.
        //
        // Important: This can bug check if the guest tries to access an MSR that is not supported by
        //            the host. See SimpleSvm for more information on how to handle this correctly.
        //
        if write_access {
            let low_part = unsafe { (*guest_context.guest_regs).rax as u32 };
            let high_part = unsafe { (*guest_context.guest_regs).rdx as u32 };

            let value = (high_part as u64) << 32 | low_part as u64;

            unsafe { wrmsr(msr, value) };
        } else {
            let value = unsafe { rdmsr(msr) };

            unsafe { (*guest_context.guest_regs).rax = (value as u32) as u64 };
            unsafe { (*guest_context.guest_regs).rdx = (value >> 32) as u64 };
        }
    }
}

pub fn handle_vmrun(data: *mut ProcessorData, _: &mut GuestContext) {
    // Inject #GP exception
    //
    EventInjection::gp().inject(data);
}

#[no_mangle]
unsafe extern "stdcall" fn handle_vmexit(
    data: *mut ProcessorData,
    guest_registers: *mut GuestRegisters,
) -> u8 {
    let mut guest_context = GuestContext::new(guest_registers, false);

    // Load host state that is not loaded on #VMEXIT.
    //
    asm!("vmload rax", in("rax") (*data).host_stack_layout.host_vmcb_pa);

    assert_eq!((*data).host_stack_layout.reserved_1, u64::MAX);

    // Guest's RAX is overwritten by the host's value on #VMEXIT and saved in
    // the VMCB instead. Reflect the guest RAX to the context.
    //
    (*guest_registers).rax = (*data).guest_vmcb.save_area.rax;

    // Update the trap frame
    //
    (*data).host_stack_layout.trap_frame.rsp = (*data).guest_vmcb.save_area.rsp;
    (*data).host_stack_layout.trap_frame.rip = (*data).guest_vmcb.control_area.nrip;

    // Handle #VMEXIT
    //
    match (*data).guest_vmcb.control_area.exit_code {
        VmExitCode::VMEXIT_CPUID => {
            handle_cpuid(data, &mut guest_context);
        }
        VmExitCode::VMEXIT_MSR => {
            handle_msr(data, &mut guest_context);
        }
        VmExitCode::VMEXIT_VMRUN => {
            handle_vmrun(data, &mut guest_context);
        }
        _ => {
            // Invalid #VMEXIT. This should never happen.
            //

            dbg_break!();

            KeBugCheck(MANUALLY_INITIATED_CRASH);
        }
    }

    // Terminate hypervisor if requested.
    //
    if guest_context.exit_vm {
        // Set return values of cpuid as follows:
        // - rbx = address to return
        // - rcx = stack pointer to restore
        //
        (*guest_context.guest_regs).rax = data as u32 as u64;
        (*guest_context.guest_regs).rdx = data as u64 >> 32;

        (*guest_context.guest_regs).rbx = (*data).guest_vmcb.control_area.nrip;
        (*guest_context.guest_regs).rcx = (*data).guest_vmcb.save_area.rsp;

        // Load guest state (currently host state is loaded)
        ////
        let guest_vmcb_pa = physical_address(&(*data).guest_vmcb as *const _ as _).as_u64();
        asm!("vmload rax", in("rax") guest_vmcb_pa);

        // Set the global interrupt flag (GIF) but still disable interrupts by
        // clearing IF. GIF must be set to return to the normal execution, but
        // interruptions are not desirable until SVM is disabled as it would
        // execute random kernel-code in the host context.
        //
        asm!("cli");
        asm!("stgi");

        // Disable svm.
        //
        let msr = rdmsr(IA32_EFER) & !EFER_SVME;
        wrmsr(IA32_EFER, msr);

        // Restore guest eflags.
        //
        // See:
        // - https://docs.microsoft.com/en-us/cpp/intrinsics/writeeflags
        // - https://www.felixcloutier.com/x86/popf:popfd:popfq
        //
        asm!("push {}; popfq", in(reg) (*data).guest_vmcb.save_area.rflags);
    }

    // Reflect potentially updated guest's RAX to VMCB. Again, unlike other GPRs,
    // RAX is loaded from VMCB on VMRUN. Afterwards, advance RIP to "complete" the
    // instruction.
    //
    (*data).guest_vmcb.save_area.rax = (*guest_context.guest_regs).rax;
    (*data).guest_vmcb.save_area.rip = (*data).guest_vmcb.control_area.nrip;

    guest_context.exit_vm as u8
}
