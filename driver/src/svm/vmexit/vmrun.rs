use crate::svm::{
    data::{guest::GuestRegisters, processor_data::ProcessorData},
    events::EventInjection,
    vmexit::ExitType,
};

pub fn handle_vmrun(data: &mut ProcessorData, _: &mut GuestRegisters) -> ExitType {
    // Inject #GP exception
    //
    EventInjection::gp().inject(data);

    ExitType::DoNothing
}
