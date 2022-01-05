use crate::debug::dbg_break;
use crate::nt::addresses::{physical_address, PhysicalAddress};
use crate::nt::include::{KeBugCheck, MANUALLY_INITIATED_CRASH};
use crate::nt::ptr::Pointer;
use crate::svm::data::guest::GuestRegisters;
use crate::svm::data::msr_bitmap::EFER_SVME;
use crate::svm::data::processor::ProcessorData;
use crate::svm::events::EventInjection;
use crate::svm::vmcb::control_area::{NptExitInfo, VmExitCode};
use core::arch::asm;

use crate::svm::paging::AccessType;
use x86::cpuid::cpuid;
use x86::msr::{rdmsr, wrmsr, IA32_EFER};

#[derive(PartialOrd, PartialEq)]
pub enum ExitType {
    ExitHypervisor,
    IncrementRIP,
    DoNothing,
}

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

pub fn handle_msr(data: &mut ProcessorData, guest_regs: &mut GuestRegisters) -> ExitType {
    let msr = guest_regs.rcx as u32;
    let write_access = data.guest_vmcb.control_area.exit_info1.bits() != 0;

    // Prevent IA32_EFER from being modified
    //
    if msr == IA32_EFER {
        #[cfg(not(feature = "no-assertions"))]
        assert!(write_access);

        let low_part = guest_regs.rax as u32;
        let high_part = guest_regs.rdx as u32;
        let value = (high_part as u64) << 32 | low_part as u64;

        // The guest is trying to enable SVM.
        //
        // Inject a #GP exception if the guest attempts to clear the flag. The
        // protection of this bit is required because clearing it would result
        // in undefined behavior.
        //
        if value & EFER_SVME != 0 {
            EventInjection::gp().inject(data);
        }

        // Otherwise, update the msr as requested.
        //
        // Note: The value should be checked beforehand to not allow any illegal values
        // and inject a #GP as needed. If that is not done, the hypervisor attempts to resume
        // the guest with an invalid EFER value and immediately receives #VMEXIT due to VMEXIT_INVALID.
        // This would in this case, result in a bug check.
        //
        // See `Extended Feature Enable Register (EFER)` for what values are allowed.
        // TODO: Implement this check
        //
        data.guest_vmcb.save_area.efer = value;
    } else {
        // Execute rdmsr or wrmsr as requested by the guest.
        //
        // Important: This can bug check if the guest tries to access an MSR that is not supported by
        //            the host. See SimpleSvm for more information on how to handle this correctly.
        //
        if write_access {
            let low_part = guest_regs.rax as u32;
            let high_part = guest_regs.rdx as u32;

            let value = (high_part as u64) << 32 | low_part as u64;

            unsafe { wrmsr(msr, value) };
        } else {
            let value = unsafe { rdmsr(msr) };

            guest_regs.rax = (value as u32) as u64;
            guest_regs.rdx = (value >> 32) as u64;
        }
    }

    ExitType::IncrementRIP
}

pub fn handle_vmrun(data: &mut ProcessorData, _: &mut GuestRegisters) -> ExitType {
    // Inject #GP exception
    //
    EventInjection::gp().inject(data);

    ExitType::DoNothing
}

pub fn handle_break_point_exception(data: &mut ProcessorData, _: &mut GuestRegisters) -> ExitType {
    // TODO:
    // - Find hook entry for the address (current rip)
    // - Set rip to the hook handler otherwise inject #BP

    EventInjection::bp().inject(data);

    ExitType::DoNothing
}

pub fn handle_nested_page_fault(data: &mut ProcessorData, _: &mut GuestRegisters) -> ExitType {
    let hooked_npt = &mut data.host_stack_layout.shared_data.hooked_npt;

    // TODO: Can we check if the guest called the hook function?
    //          If not, what should we do?
    //          If yes, we can parse the RET address.
    //
    // TODO: Make sure there's no way to scan physical memory to find the hook
    //     - We have to map hook_pa in the guest to something else -> Use a physical page that is > 512GB maybe.
    //
    // TODO: Could we intercept page reads via RW error code?

    // From the AMD manual: `15.25.6 Nested versus Guest Page Faults, Fault Ordering`
    //
    // Nested page faults are entirely a function of the nested page table and VMM processor mode. Nested
    // faults cause a #VMEXIT(NPF) to the VMM. The faulting guest physical address is saved in the
    // VMCB's EXITINFO2 field; EXITINFO1 delivers an error code similar to a #PF error code.
    //
    let faulting_pa = data.guest_vmcb.control_area.exit_info2;
    let exit_info = data.guest_vmcb.control_area.exit_info1;

    // Page was not present so we have to map it.
    //
    if !exit_info.contains(NptExitInfo::PRESENT) {
        let faulting_pa = PhysicalAddress::from_pa(faulting_pa)
            .align_down_to_base_page()
            .as_u64();

        hooked_npt
            .npt
            .map_4kb(faulting_pa, faulting_pa, AccessType::ReadWriteExecute);

        return ExitType::DoNothing;
    }

    // Check if there exists a hook for the faulting page.
    // - #1 - Yes: Guest tried to execute a function inside the hooked page.
    // - #2 - No: Guest tried to execute code outside the hooked page (our hook has been exited).
    //
    // For both these situations, we have to do something different:
    // - #1 - Change permission of hook page to RWX and of other pages to RW.
    // - #2 - Change permission of hook page to RW and of other pages to RWX.
    //

    // TODO: Make a diagram of this.

    // dbg_break!();

    // We need to do 2 things:
    // - Change the permissions of the hooked page to RWX
    // - Detect when the hooked page goes out of scope to restore the permissions.
    //
    if let Some(hook_pa) = hooked_npt
        .find_hook(faulting_pa)
        .map(|hook| hook.hook_pa.as_u64())
    {
        let stack =
            unsafe { core::slice::from_raw_parts(data.guest_vmcb.save_area.rsp as *mut u64, 10) };
        let return_address = stack[0];
        let return_address = PhysicalAddress::from_va(return_address)
            .align_down_to_base_page()
            .as_u64();

        hooked_npt
            .npt
            .change_page_permission(faulting_pa, hook_pa, AccessType::ReadWriteExecute);

        let _ = hooked_npt.npt.split_2mb_to_4kb(return_address);
        hooked_npt.npt.change_page_permission(
            return_address,
            return_address,
            AccessType::ReadWrite,
        );
    } else {
        // We hit the rw return address. Time to hide all our hooks.
        //
        hooked_npt.hide_hooks();

        // Also make the current address (the previous return address) executable again.
        //
        hooked_npt.npt.change_page_permission(
            faulting_pa,
            faulting_pa,
            AccessType::ReadWriteExecute,
        );
    }

    // Apply or revert hooks
    //
    // let hide_hook = || {
    //     map_all(rwx);
    //     map_4k(guest_pa, guest_pa, rw); // revert the hook
    // };
    // let show_hook = || {
    //     map_all(rw);
    //     map_4k(guest_pa, hook.pa, rwx);  // apply hook
    // };

    // if there's a hook for this address
    //
    // if let Some(hook) = find_hook(rip?) {
    //     if let Some(hook) = active_hook() {
    //         // Hooked page jumped to hooked page
    //         // TODO: This could be optimized, but it shouldn't be used often.
    //         hide_hook();
    //         show_hook();
    //     } else {
    //         // Legit page jumped to hooked page
    //         show_hook();
    //     }
    // } else {
    //     // Hooked page jumped outside. Hide hook.
    //     hide_hook();
    // }

    ExitType::DoNothing
}

unsafe fn exit_hypervisor(data: &mut ProcessorData, guest_regs: &mut GuestRegisters) {
    // Set return values of cpuid as follows:
    // - rbx = address to return
    // - rcx = stack pointer to restore
    //
    guest_regs.rax = data as *mut _ as u32 as u64;
    guest_regs.rdx = data as *mut _ as u64 >> 32;

    guest_regs.rbx = data.guest_vmcb.control_area.nrip;
    guest_regs.rcx = data.guest_vmcb.save_area.rsp;

    // Load guest state (currently host state is loaded)
    ////
    let guest_vmcb_pa = physical_address(&data.guest_vmcb as *const _ as _).as_u64();
    asm!("vmload rax", in("rax") guest_vmcb_pa);

    // Set the global interrupt flag (GIF) but still disable interrupts by
    // clearing IF. GIF must be set to return to the normal execution, but
    // interruptions are not desirable until SVM is disabled as it would
    // execute random kernel-code in the host context.
    //
    asm!("cli");
    asm!("stgi");

    // Disable svm.
    //
    let msr = rdmsr(IA32_EFER) & !EFER_SVME;
    wrmsr(IA32_EFER, msr);

    // Restore guest eflags.
    //
    // See:
    // - https://docs.microsoft.com/en-us/cpp/intrinsics/writeeflags
    // - https://www.felixcloutier.com/x86/popf:popfd:popfq
    //
    asm!("push {}; popfq", in(reg) (*data).guest_vmcb.save_area.rflags);
}

#[no_mangle]
unsafe extern "stdcall" fn handle_vmexit(
    mut data: Pointer<ProcessorData>,
    mut guest_regs: Pointer<GuestRegisters>,
) -> u8 {
    // Load host state that is not loaded on #VMEXIT.
    //
    asm!("vmload rax", in("rax") data.host_stack_layout.host_vmcb_pa);

    // #[cfg(not(feature = "no-assertions"))]
    assert_eq!(data.host_stack_layout.reserved_1, u64::MAX);

    // Guest's RAX is overwritten by the host's value on #VMEXIT and saved in
    // the VMCB instead. Reflect the guest RAX to the context.
    //
    guest_regs.rax = data.guest_vmcb.save_area.rax;

    // Update the trap frame
    //
    data.host_stack_layout.trap_frame.rsp = data.guest_vmcb.save_area.rsp;
    data.host_stack_layout.trap_frame.rip = data.guest_vmcb.control_area.nrip;

    // Handle #VMEXIT
    //
    let exit_type = match data.guest_vmcb.control_area.exit_code {
        VmExitCode::VMEXIT_CPUID => handle_cpuid(&mut data, &mut guest_regs),
        VmExitCode::VMEXIT_MSR => handle_msr(&mut data, &mut guest_regs),
        VmExitCode::VMEXIT_VMRUN => handle_vmrun(&mut data, &mut guest_regs),
        VmExitCode::VMEXIT_EXCEPTION_BP => handle_break_point_exception(&mut data, &mut guest_regs),
        VmExitCode::VMEXIT_NPF => handle_nested_page_fault(&mut data, &mut guest_regs),
        _ => {
            // Invalid #VMEXIT. This should never happen.
            //

            dbg_break!();

            KeBugCheck(MANUALLY_INITIATED_CRASH);
        }
    };

    // Handle the exit status of the vmexit handlers
    //
    match exit_type {
        ExitType::ExitHypervisor => exit_hypervisor(&mut data, &mut guest_regs),
        ExitType::IncrementRIP => {
            // Reflect potentially updated guest's RAX to VMCB. Again, unlike other GPRs,
            // RAX is loaded from VMCB on VMRUN. Afterwards, advance RIP to "complete" the
            // instruction.
            //
            data.guest_vmcb.save_area.rax = guest_regs.rax;
            data.guest_vmcb.save_area.rip = data.guest_vmcb.control_area.nrip;
        }
        ExitType::DoNothing => {}
    }

    (exit_type == ExitType::ExitHypervisor) as u8
}
