use crate::{
    svm::{
        data::{guest::GuestRegisters, processor_data::ProcessorData},
        events::EventInjection,
        paging::AccessType,
        vmcb::control_area::NptExitInfo,
        vmexit::ExitType,
    },
    utils::addresses::PhysicalAddress,
};

pub fn handle_default(data: &mut ProcessorData, _regs: &mut GuestRegisters) -> ExitType {
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
            .secondary_npt
            .map_4kb(faulting_pa, faulting_pa, AccessType::ReadWrite);
        hooked_npt
            .primary_npt
            .map_4kb(faulting_pa, faulting_pa, AccessType::ReadWriteExecute);

        ExitType::Continue
    } else {
        EventInjection::pf().inject(data);

        ExitType::Continue
    }
}
