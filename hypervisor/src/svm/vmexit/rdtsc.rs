use crate::svm::{utils::guest::GuestRegs, vcpu_data::VcpuData, vmexit::ExitType};
use x86::time::rdtsc;

pub fn handle_default(_vcpu: &mut VcpuData, regs: &mut GuestRegs) -> ExitType {
    let rdtsc = unsafe { rdtsc() };

    regs.rax = (rdtsc as u32) as u64;
    regs.rdx = (rdtsc >> 32) as u64;

    ExitType::IncrementRIP
}
