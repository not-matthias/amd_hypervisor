use crate::debug::dbg_break;
use crate::nt::include::{KeBugCheck, MANUALLY_INITIATED_CRASH};
use crate::svm::data::guest::{GuestContext, GuestRegisters};
use crate::svm::data::processor::ProcessorData;
use crate::svm::events::EventInjection;
use crate::svm::vmcb::control_area::VmExitCode;
use core::arch::asm;
use x86::cpuid::cpuid;
use x86::msr::{rdmsr, wrmsr};

pub fn handle_cpuid(data: *mut ProcessorData, guest_context: &mut GuestContext) {
    // Execute cpuid as requested
    //
    let leaf = unsafe { (*guest_context.guest_regs).rax };
    let subleaf = unsafe { (*guest_context.guest_regs).rcx };

    let cpuid = cpuid!(leaf, subleaf);

    // Modify certain leafs
    //
    // TODO: implement

    // Store the result
    //
    unsafe {
        (*guest_context.guest_regs).rax = cpuid.eax as u64;
        (*guest_context.guest_regs).rbx = cpuid.ebx as u64;
        (*guest_context.guest_regs).rcx = cpuid.ecx as u64;
        (*guest_context.guest_regs).rdx = cpuid.edx as u64;
    }

    // Then, advance RIP to "complete" the instruction.
    //
    unsafe { (*data).guest_vmcb.save_area.rip = (*data).guest_vmcb.control_area.nrip };
}

pub fn handle_msr(data: *mut ProcessorData, guest_context: &mut GuestContext) {
    let msr = unsafe { (*guest_context.guest_regs).rcx as u32 };
    let write_access = unsafe { (*data).guest_vmcb.control_area.exit_info1 } != 0;

    dbg_break!();

    // Prevent IA32_EFER from being modified
    //
    // if msr == IA32_EFER {
    //     // TODO: Implement
    //     //
    // } else {
    //     //
    //     //
    // }

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

        // TODO: Check if `value as u32` is the same as `value & u32::MAX`

        unsafe { (*guest_context.guest_regs).rax = (value as u32) as u64 };
        unsafe { (*guest_context.guest_regs).rdx = (value >> 32) as u64 };
    }

    // Then, advance RIP to "complete" the instruction.
    //
    unsafe { (*data).guest_vmcb.save_area.rip = (*data).guest_vmcb.control_area.nrip };
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

            dbg_break!();

            KeBugCheck(MANUALLY_INITIATED_CRASH);
        }
    }

    // Terminate hypervisor if requested. TODO: Implement

    // Reflect potentially updated guest's RAX to VMCB. Again, unlike other GPRs,
    // RAX is loaded from VMCB on VMRUN.
    //
    (*data).guest_vmcb.save_area.rax = (*guest_context.guest_regs).rax;

    // Return whether or not we should exit the virtual machine.
    //
    guest_context.exit_vm as u8
}
