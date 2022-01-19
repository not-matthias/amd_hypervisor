use hypervisor::svm::{
    data::{guest::GuestRegisters, processor_data::ProcessorData},
    vmexit::ExitType,
};
use x86::cpuid::cpuid;

pub const CPUID_FEATURES: u32 = 0x1;
pub fn handle_features(_vcpu: &mut ProcessorData, regs: &mut GuestRegisters) -> ExitType {
    let mut cpuid = cpuid!(regs.rax, regs.rcx);

    // Indicate presence of a hypervisor by setting the bit that are
    // reserved for use by hypervisor to indicate guest status. See
    // "CPUID Fn0000_0001_ECX Feature Identifiers".
    //
    const CPUID_FN0000_0001_ECX_HYPERVISOR_PRESENT: u32 = 1 << 31;
    // cpuid.ecx |= CPUID_FN0000_0001_ECX_HYPERVISOR_PRESENT;

    // Actually, we don't do that here. We'll hide it, so that the guest won't know
    // that it's running with a hypervisor.
    //
    cpuid.ecx &= !CPUID_FN0000_0001_ECX_HYPERVISOR_PRESENT;

    // Write the result back to the guest.
    //
    regs.rax = cpuid.eax as u64;
    regs.rbx = cpuid.ebx as u64;
    regs.rcx = cpuid.ecx as u64;
    regs.rdx = cpuid.edx as u64;

    ExitType::IncrementRIP
}

pub const CPUID_HV_VENDOR: u32 = 0x40000000;
pub fn handle_hv_vendor(_vcpu: &mut ProcessorData, regs: &mut GuestRegisters) -> ExitType {
    regs.rax = 0x0;
    regs.rbx = 0x0;
    regs.rcx = 0x0;
    regs.rdx = 0x0;

    ExitType::IncrementRIP
}
