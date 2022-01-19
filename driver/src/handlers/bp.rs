use hypervisor::{
    hook::HookType,
    svm::{
        data::{guest::GuestRegisters, processor_data::ProcessorData},
        events::EventInjection,
        vmexit::ExitType,
    },
};

pub fn handle_bp_exception(vcpu: &mut ProcessorData, _: &mut GuestRegisters) -> ExitType {
    let hooked_npt = unsafe { &mut vcpu.host_stack_layout.shared_data.as_mut().hooked_npt };

    // Find the handler address for the current instruction pointer (RIP) and
    // transfer the execution to it. If we couldn't find a hook, we inject the
    // #BP exception.
    //
    if let Some(Some(handler)) = hooked_npt
        .find_hook_by_address(vcpu.guest_vmcb.save_area.rip)
        .map(|hook| {
            if let HookType::Function { inline_hook } = &hook.hook_type {
                Some(inline_hook.handler_address())
            } else {
                None
            }
        })
    {
        vcpu.guest_vmcb.save_area.rip = handler;

        ExitType::Continue
    } else {
        EventInjection::bp().inject(vcpu);

        ExitType::IncrementRIP
    }
}
