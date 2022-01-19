use hypervisor::{
    svm::{
        data::{guest::GuestRegisters, processor_data::ProcessorData},
        paging::AccessType,
        vmcb::control_area::{NptExitInfo, TlbControl, VmcbClean},
        vmexit::ExitType,
    },
    utils::addresses::PhysicalAddress,
};

pub fn handle_npf(vcpu: &mut ProcessorData, _regs: &mut GuestRegisters) -> ExitType {
    let hooked_npt = unsafe { &mut vcpu.host_stack_layout.shared_data.as_mut().hooked_npt };

    // From the AMD manual: `15.25.6 Nested versus Guest Page Faults, Fault
    // Ordering`
    //
    // Nested page faults are entirely a function of the nested page table and VMM
    // processor mode. Nested faults cause a #VMEXIT(NPF) to the VMM. The
    // faulting guest physical address is saved in the VMCB's EXITINFO2 field;
    // EXITINFO1 delivers an error code similar to a #PF error code.
    //
    let faulting_pa = vcpu.guest_vmcb.control_area.exit_info2;
    let exit_info = vcpu.guest_vmcb.control_area.exit_info1;

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

        return ExitType::Continue;
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

        vcpu.guest_vmcb.control_area.ncr3 = hooked_npt.rw_pml4.as_u64();
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

        vcpu.guest_vmcb.control_area.ncr3 = hooked_npt.rwx_pml4.as_u64();
    }

    // We changed the `cr3` of the guest, so we have to flush the TLB.
    //
    vcpu.guest_vmcb
        .control_area
        .tlb_control
        .insert(TlbControl::FLUSH_GUEST_TLB);
    vcpu.guest_vmcb
        .control_area
        .vmcb_clean
        .remove(VmcbClean::NP);

    ExitType::Continue
}
