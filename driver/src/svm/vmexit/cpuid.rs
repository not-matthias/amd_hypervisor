use crate::svm::{
    data::{guest::GuestRegisters, processor_data::ProcessorData},
    vmexit::ExitType,
};
use x86::cpuid::cpuid;

pub const CPUID_DEVIRTUALIZE: u64 = 0xBEAC_0000;
pub const CPUID_IS_INSTALLED: u64 = 0xBEAC_0001;

pub fn handle_cpuid(_data: &mut ProcessorData, guest_regs: &mut GuestRegisters) -> ExitType {
    // Execute cpuid as requested
    //
    let leaf = guest_regs.rax;
    let subleaf = guest_regs.rcx;

    let mut cpuid = cpuid!(leaf, subleaf);

    // Modify certain leafs
    //
    const CPUID_PROCESSOR_AND_PROCESSOR_FEATURE_IDENTIFIERS: u64 = 0x00000001;
    const CPUID_FN0000_0001_ECX_HYPERVISOR_PRESENT: u32 = 1 << 31;
    const CPUID_HV_VENDOR_AND_MAX_FUNCTIONS: u64 = 0x40000000;

    // TODO: Do we have to manually toggle the hv present bit?

    match leaf {
        CPUID_PROCESSOR_AND_PROCESSOR_FEATURE_IDENTIFIERS => {
            // Indicate presence of a hypervisor by setting the bit that are
            // reserved for use by hypervisor to indicate guest status. See
            // "CPUID Fn0000_0001_ECX Feature Identifiers".
            //
            // Actually, we don't do that here. We'll hide it, so that the guest won't know
            // that it's running with a hypervisor.
            //
            cpuid.ecx &= !CPUID_FN0000_0001_ECX_HYPERVISOR_PRESENT;
        }
        CPUID_HV_VENDOR_AND_MAX_FUNCTIONS => {
            cpuid.eax = 0x0;
            cpuid.ebx = 0x0;
            cpuid.ecx = 0x0;
            cpuid.edx = 0x0;
        }
        CPUID_IS_INSTALLED => {
            cpuid.eax = 0x42;
            cpuid.ebx = 0x42;
            cpuid.ecx = 0x42;
            cpuid.edx = 0x42;
        }
        CPUID_DEVIRTUALIZE => {
            return ExitType::ExitHypervisor;
        }
        _ => {}
    }

    // Store the result
    //
    guest_regs.rax = cpuid.eax as u64;
    guest_regs.rbx = cpuid.ebx as u64;
    guest_regs.rcx = cpuid.ecx as u64;
    guest_regs.rdx = cpuid.edx as u64;

    ExitType::IncrementRIP
}
