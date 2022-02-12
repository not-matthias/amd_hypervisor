use crate::HOOK_MANAGER;
use hypervisor::{
    svm::{
        utils::{guest::GuestRegs, paging::AccessType},
        vcpu_data::VcpuData,
        vmcb::control_area::{NptExitInfo, TlbControl},
        vmexit::ExitType,
    },
    utils::addresses::PhysicalAddress,
};

pub fn handle_npf(vcpu: &mut VcpuData, _regs: &mut GuestRegs) -> ExitType {
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
        let pa = PhysicalAddress::from_pa(faulting_pa)
            .align_down_to_base_page()
            .as_u64();

        vcpu.shared_data()
            .secondary_npt
            .map_4kb(pa, pa, AccessType::ReadWrite);
        vcpu.shared_data()
            .primary_npt
            .map_4kb(pa, pa, AccessType::ReadWriteExecute);

        return ExitType::Continue;
    }

    // Check if there exists a hook for the faulting page.
    // - #1 - Yes: Guest tried to execute a function inside the hooked page.
    // - #2 - No: Guest tried to execute code outside the hooked page (our hook has
    //   finished executing).
    //
    let hook_manager = unsafe { HOOK_MANAGER.as_ref().unwrap() };
    if let Some(hook_pa) = hook_manager
        .find_hook(faulting_pa)
        .map(|hook| hook.page_pa.as_u64())
    {
        vcpu.shared_data().secondary_npt.remap_page(
            faulting_pa,
            hook_pa,
            AccessType::ReadWriteExecute,
        );

        vcpu.guest_vmcb.control_area.ncr3 = vcpu.shared_data().secondary_pml4.as_u64();
    } else {
        // Just to be safe: Change the permission of the faulting pa to rwx again. I'm
        // not sure why we need this, but if we don't do it, we'll get stuck at
        // 'Launching vm'.
        //
        // TODO: Check if we need this.
        vcpu.shared_data().primary_npt.remap_page(
            faulting_pa,
            faulting_pa,
            AccessType::ReadWriteExecute,
        );

        vcpu.guest_vmcb.control_area.ncr3 = vcpu.shared_data().primary_pml4.as_u64();
    }

    // We changed the `cr3` of the guest, so we have to flush the TLB.
    //
    // Note: If you have an older cpu (or if you are running inside kvm),
    // `FLUSH_GUEST_TLB` might not be supported. If that's the case, we have to use
    // `FLUSH_ENTIRE_TLB`.
    //
    vcpu.guest_vmcb
        .control_area
        .tlb_control
        .insert(TlbControl::FLUSH_GUEST_TLB);

    ExitType::Continue
}
