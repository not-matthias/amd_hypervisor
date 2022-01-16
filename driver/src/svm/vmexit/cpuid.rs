use crate::svm::{
    data::{guest::GuestRegisters, processor_data::ProcessorData},
    vmexit::ExitType,
};
use x86::cpuid::cpuid;

pub const CPUID_DEVIRTUALIZE: u64 = 0x41414141;

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
    const CPUID_HV_MAX: u32 = CPUID_HV_INTERFACE as u32;

    const CPUID_HV_INTERFACE: u64 = 0x40000001;

    match leaf {
        CPUID_PROCESSOR_AND_PROCESSOR_FEATURE_IDENTIFIERS => {
            // Indicate presence of a hypervisor by setting the bit that are
            // reserved for use by hypervisor to indicate guest status. See "CPUID
            // Fn0000_0001_ECX Feature Identifiers".
            //
            cpuid.ecx |= CPUID_FN0000_0001_ECX_HYPERVISOR_PRESENT;
        }
        CPUID_HV_VENDOR_AND_MAX_FUNCTIONS => {
            cpuid.eax = CPUID_HV_MAX;
            cpuid.ebx = 0x42;
            cpuid.ecx = 0x42;
            cpuid.edx = 0x42;
        }
        CPUID_HV_INTERFACE => {
            // Return non Hv#1 value. This indicate that the SimpleSvm does NOT
            // conform to the Microsoft hypervisor interface.
            //
            cpuid.eax = u32::from_le_bytes(*b"0#vH"); // Hv#0
            cpuid.ebx = 0;
            cpuid.ecx = 0;
            cpuid.edx = 0;
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
