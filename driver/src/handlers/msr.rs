use crate::handlers::rdtsc::RDTSC_MODIFIER;
use hypervisor::svm::{
    data::{guest::GuestRegisters, processor_data::ProcessorData},
    vmexit::ExitType,
};
use x86::time::rdtsc;

pub fn handle_hsave(_vcpu: &mut ProcessorData, regs: &mut GuestRegisters) -> ExitType {
    regs.rax = 0;
    regs.rdx = 0;

    ExitType::IncrementRIP
}

pub fn handle_rdtsc(_vcpu: &mut ProcessorData, regs: &mut GuestRegisters) -> ExitType {
    let rdtsc = unsafe { rdtsc() };
    let rdtsc = rdtsc / RDTSC_MODIFIER;

    regs.rax = (rdtsc as u32) as u64;
    regs.rdx = (rdtsc >> 32) as u64;

    ExitType::IncrementRIP
}
