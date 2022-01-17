use crate::svm::{
    data::{guest::GuestRegisters, processor_data::ProcessorData},
    vmexit::ExitType,
};
use x86::time::rdtsc;

pub const RDTSC_MODIFIER: u64 = 100;

pub fn handle_rdtsc(_data: &mut ProcessorData, regs: &mut GuestRegisters) -> ExitType {
    let rdtsc = unsafe { rdtsc() };
    let rdtsc = (rdtsc / RDTSC_MODIFIER) as i64;

    regs.rax = (rdtsc & -1) as u64;
    regs.rdx = ((rdtsc >> 32) & -1) as u64;

    ExitType::IncrementRIP
}
