use crate::HOOK_MANAGER;
use hypervisor::{
    hook::HookType,
    svm::{
        data::{guest::GuestRegisters, processor_data::ProcessorData},
        events::EventInjection,
        vmexit::ExitType,
    },
};

pub fn handle_bp_exception(vcpu: &mut ProcessorData, _: &mut GuestRegisters) -> ExitType {
    let hook_manager = unsafe { HOOK_MANAGER.as_ref().unwrap() };

    // Find the handler address for the current instruction pointer (RIP) and
    // transfer the execution to it. If we couldn't find a hook, we inject the
    // #BP exception.
    //
    if let Some(Some(handler)) = hook_manager
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
