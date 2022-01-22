use hypervisor::svm::{
    data::{guest::GuestRegisters, processor_data::ProcessorData},
    vmexit::ExitType,
};
use x86::time::rdtsc;

pub const RDTSC_MODIFIER: u64 = 100;

pub fn handle_rdtsc(_vcpu: &mut ProcessorData, regs: &mut GuestRegisters) -> ExitType {
    let rdtsc = unsafe { rdtsc() };

    regs.rax = (rdtsc as u32) as u64;
    regs.rdx = (rdtsc >> 32) as u64;

    ExitType::IncrementRIP
}
