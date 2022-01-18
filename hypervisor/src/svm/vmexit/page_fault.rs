use crate::{
    hook::HookType,
    svm::{
        data::{guest::GuestRegisters, processor_data::ProcessorData},
        events::EventInjection,
        paging::AccessType,
        vmcb::control_area::{NptExitInfo, TlbControl, VmcbClean},
        vmexit::ExitType,
    },
    utils::addresses::PhysicalAddress,
};

pub fn handle_break_point_exception(data: &mut ProcessorData, _: &mut GuestRegisters) -> ExitType {
    let hooked_npt = unsafe { &mut data.host_stack_layout.shared_data.as_mut().hooked_npt };

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
    let hooked_npt = unsafe { &mut data.host_stack_layout.shared_data.as_mut().hooked_npt };

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
