use crate::{
    debug::dbg_break,
    nt::{
        addresses::{physical_address, PhysicalAddress},
        include::{KeBugCheck, MANUALLY_INITIATED_CRASH},
        ptr::Pointer,
    },
    svm::{
        data::{guest::GuestRegisters, msr_bitmap::EFER_SVME, processor_data::ProcessorData},
        events::EventInjection,
        paging::AccessType,
        vmcb::control_area::{NptExitInfo, TlbControl, VmExitCode, VmcbClean},
    },
    HookType,
};
use core::arch::asm;
use x86::{
    cpuid::cpuid,
    msr::{rdmsr, wrmsr, IA32_EFER},
};

#[derive(PartialOrd, PartialEq)]
pub enum ExitType {
    ExitHypervisor,
    IncrementRIP,
    DoNothing,
}

pub const CPUID_DEVIRTUALIZE: u64 = 0x41414141;

pub fn handle_cpuid(_data: &mut ProcessorData, guest_regs: &mut GuestRegisters) -> ExitType {
    // Execute cpuid as requested
    //
    let leaf = guest_regs.rax;
    let subleaf = guest_regs.rcx;

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
            return ExitType::ExitHypervisor;
        }
        _ => {}
    }

    // Store the result
    //
    guest_regs.rax = cpuid.eax as u64;
    guest_regs.rbx = cpuid.ebx as u64;
    guest_regs.rcx = cpuid.ecx as u64;
    guest_regs.rdx = cpuid.edx as u64;

    ExitType::IncrementRIP
}

pub fn handle_msr(data: &mut ProcessorData, guest_regs: &mut GuestRegisters) -> ExitType {
    let msr = guest_regs.rcx as u32;
    let write_access = data.guest_vmcb.control_area.exit_info1.bits() != 0;

    // Prevent IA32_EFER from being modified
    //
    if msr == IA32_EFER {
        #[cfg(not(feature = "no-assertions"))]
        assert!(write_access);

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

pub fn handle_vmrun(data: &mut ProcessorData, _: &mut GuestRegisters) -> ExitType {
    // Inject #GP exception
    //
    EventInjection::gp().inject(data);

    ExitType::DoNothing
}

pub fn handle_break_point_exception(data: &mut ProcessorData, _: &mut GuestRegisters) -> ExitType {
    let hooked_npt = &mut data.host_stack_layout.shared_data.hooked_npt;

    // Find the handler address for the current instruction pointer (RIP) and
    // transfer the execution to it. If we couldn't find a hook, we inject the
    // #BP exception.
    //
    if let Some(Some(handler)) = hooked_npt
        .find_hook_by_address(data.guest_vmcb.save_area.rip)
        .map(|hook| {
            if let HookType::Function { inline_hook } = &hook.hook_type {
                Some(inline_hook.handler_address())
            } else {
                None
            }
        })
    {
        data.guest_vmcb.save_area.rip = handler;

        ExitType::DoNothing
    } else {
        EventInjection::bp().inject(data);

        ExitType::IncrementRIP
    }
}

pub fn handle_nested_page_fault(data: &mut ProcessorData, _regs: &mut GuestRegisters) -> ExitType {
    let hooked_npt = &mut data.host_stack_layout.shared_data.hooked_npt;

    // From the AMD manual: `15.25.6 Nested versus Guest Page Faults, Fault
    // Ordering`
    //
    // Nested page faults are entirely a function of the nested page table and VMM
    // processor mode. Nested faults cause a #VMEXIT(NPF) to the VMM. The
    // faulting guest physical address is saved in the VMCB's EXITINFO2 field;
    // EXITINFO1 delivers an error code similar to a #PF error code.
    //
    let faulting_pa = data.guest_vmcb.control_area.exit_info2;
    let exit_info = data.guest_vmcb.control_area.exit_info1;

    // Page was not present so we have to map it.
    //
    if !exit_info.contains(NptExitInfo::PRESENT) {
        let faulting_pa = PhysicalAddress::from_pa(faulting_pa)
            .align_down_to_base_page()
            .as_u64();

        hooked_npt
            .rw_npt
            .map_4kb(faulting_pa, faulting_pa, AccessType::ReadWrite);
        hooked_npt
            .rwx_npt
            .map_4kb(faulting_pa, faulting_pa, AccessType::ReadWriteExecute);

        return ExitType::DoNothing;
    }

    // Check if there exists a hook for the faulting page.
    // - #1 - Yes: Guest tried to execute a function inside the hooked page.
    // - #2 - No: Guest tried to execute code outside the hooked page (our hook has
    //   been exited).
    //
    if let Some(hook_pa) = hooked_npt
        .find_hook(faulting_pa)
        .map(|hook| hook.page_pa.as_u64())
    {
        hooked_npt.rw_npt.change_page_permission(
            faulting_pa,
            hook_pa,
            AccessType::ReadWriteExecute,
        );

        data.guest_vmcb.control_area.ncr3 = hooked_npt.rw_pml4.as_u64();
    } else {
        // Just to be safe: Change the permission of the faulting pa to rwx again. I'm
        // not sure why we need this, but if we don't do it, we'll get stuck at
        // 'Launching vm'.
        //
        hooked_npt.rwx_npt.change_page_permission(
            faulting_pa,
            faulting_pa,
            AccessType::ReadWriteExecute,
        );

        data.guest_vmcb.control_area.ncr3 = hooked_npt.rwx_pml4.as_u64();
    }

    // We changed the `cr3` of the guest, so we have to flush the TLB.
    //
    data.guest_vmcb
        .control_area
        .tlb_control
        .insert(TlbControl::FLUSH_GUEST_TLB);
    data.guest_vmcb
        .control_area
        .vmcb_clean
        .remove(VmcbClean::NP);

    ExitType::DoNothing
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
