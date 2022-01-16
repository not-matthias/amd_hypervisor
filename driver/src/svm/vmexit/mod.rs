use crate::{
    debug::dbg_break,
    svm::{
        data::{guest::GuestRegisters, msr_bitmap::EFER_SVME, processor_data::ProcessorData},
        vmcb::control_area::{VmExitCode},
        vmexit::{
            cpuid::handle_cpuid,
            msr::handle_msr,
            page_fault::{handle_break_point_exception, handle_nested_page_fault},
            vmrun::handle_vmrun,
        },
    },
    utils::{
        addresses::{physical_address},
        nt::{KeBugCheck, MANUALLY_INITIATED_CRASH},
        ptr::Pointer,
    },
};
use core::arch::asm;
use x86::{
    msr::{rdmsr, wrmsr, IA32_EFER},
};

pub mod cpuid;
pub mod msr;
pub mod page_fault;
pub mod vmrun;

#[derive(PartialOrd, PartialEq)]
pub enum ExitType {
    ExitHypervisor,
    IncrementRIP,
    DoNothing,
}

unsafe fn exit_hypervisor(data: &mut ProcessorData, guest_regs: &mut GuestRegisters) {
    // Set return values of cpuid as follows:
    // - rbx = address to return
    // - rcx = stack pointer to restore
    //
    guest_regs.rax = data as *mut _ as u32 as u64;
    guest_regs.rdx = data as *mut _ as u64 >> 32;

    guest_regs.rbx = data.guest_vmcb.control_area.nrip;
    guest_regs.rcx = data.guest_vmcb.save_area.rsp;

    // Load guest state (currently host state is loaded)
    ////
    let guest_vmcb_pa = physical_address(&data.guest_vmcb as *const _ as _).as_u64();
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

#[no_mangle]
unsafe extern "stdcall" fn handle_vmexit(
    mut data: Pointer<ProcessorData>, mut guest_regs: Pointer<GuestRegisters>,
) -> u8 {
    // Load host state that is not loaded on #VMEXIT.
    //
    asm!("vmload rax", in("rax") data.host_stack_layout.host_vmcb_pa);

    #[cfg(not(feature = "no-assertions"))]
    assert_eq!(data.host_stack_layout.reserved_1, u64::MAX);

    // Guest's RAX is overwritten by the host's value on #VMEXIT and saved in
    // the VMCB instead. Reflect the guest RAX to the context.
    //
    guest_regs.rax = data.guest_vmcb.save_area.rax;

    // Update the trap frame
    //
    data.host_stack_layout.trap_frame.rsp = data.guest_vmcb.save_area.rsp;
    data.host_stack_layout.trap_frame.rip = data.guest_vmcb.control_area.nrip;

    // Handle #VMEXIT
    //
    let exit_type = match data.guest_vmcb.control_area.exit_code {
        VmExitCode::VMEXIT_CPUID => handle_cpuid(&mut data, &mut guest_regs),
        VmExitCode::VMEXIT_MSR => handle_msr(&mut data, &mut guest_regs),
        VmExitCode::VMEXIT_VMRUN => handle_vmrun(&mut data, &mut guest_regs),
        VmExitCode::VMEXIT_EXCEPTION_BP => handle_break_point_exception(&mut data, &mut guest_regs),
        VmExitCode::VMEXIT_NPF => handle_nested_page_fault(&mut data, &mut guest_regs),
        _ => {
            // Invalid #VMEXIT. This should never happen.
            //

            dbg_break!();

            KeBugCheck(MANUALLY_INITIATED_CRASH);
        }
    };

    // Handle the exit status of the vmexit handlers
    //
    match exit_type {
        ExitType::ExitHypervisor => exit_hypervisor(&mut data, &mut guest_regs),
        ExitType::IncrementRIP => {
            // Reflect potentially updated guest's RAX to VMCB. Again, unlike other GPRs,
            // RAX is loaded from VMCB on VMRUN. Afterwards, advance RIP to "complete" the
            // instruction.
            //
            data.guest_vmcb.save_area.rax = guest_regs.rax;
            data.guest_vmcb.save_area.rip = data.guest_vmcb.control_area.nrip;
        }
        ExitType::DoNothing => {}
    }

    (exit_type == ExitType::ExitHypervisor) as u8
}
