use crate::{
    debug::dbg_break,
    svm::{
        data::{guest::GuestRegisters, processor_data::ProcessorData},
        events::EventInjection,
        msr::EFER_SVME,
        vmcb::control_area::VmExitCode,
        vmexit::cpuid::CPUID_DEVIRTUALIZE,
        VmExitType,
    },
    utils::{
        addresses::physical_address,
        nt::{KeBugCheck, MANUALLY_INITIATED_CRASH},
    },
};
use core::{arch::asm, ptr::NonNull};
use fnv::FnvBuildHasher;
use hashbrown::HashMap;
use lazy_static::lazy_static;
use spin::RwLock;
use x86::msr::{rdmsr, wrmsr, IA32_EFER};

pub mod cpuid;
pub mod msr;
pub mod npt;

pub type VmExitHandler = fn(&mut ProcessorData, &mut GuestRegisters) -> ExitType;

lazy_static! {
    pub static ref VMEXIT_HANDLERS: RwLock<HashMap<VmExitType, VmExitHandler, FnvBuildHasher>> = {
        let map = RwLock::new(HashMap::with_hasher(FnvBuildHasher::default()));

        // Default implementations
        //
        macro add_handler($vmexit_type: expr, $handler: expr) {
            let _ = map.write().insert($vmexit_type, $handler as VmExitHandler);
        }

        add_handler!(VmExitType::Msr(IA32_EFER), msr::handle_efer);
        add_handler!(VmExitType::Cpuid(CPUID_DEVIRTUALIZE), cpuid::handle_devirtualize);

        map
    };
}

pub macro vmexit_installed($vmexit_type:pat) {
    $crate::svm::vmexit::VMEXIT_HANDLERS
        .read()
        .iter()
        .any(|(key, _)| matches!(key, $vmexit_type))
}

#[derive(PartialOrd, PartialEq)]
pub enum ExitType {
    ExitHypervisor,
    IncrementRIP,
    Continue,
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
    mut data: NonNull<ProcessorData>, mut guest_regs: NonNull<GuestRegisters>,
) -> u8 {
    let data = data.as_mut();
    let guest_regs = guest_regs.as_mut();

    // Load host state that is not loaded on #VMEXIT.
    //
    asm!("vmload rax", in("rax") data.host_stack_layout.host_vmcb_pa);
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
    macro_rules! call_handler {
        ($handler:expr) => {
            if let Some(handler) = VMEXIT_HANDLERS.read().get(&$handler) {
                handler(data, guest_regs)
            } else {
                unreachable!()
            }
        };

        ($handler:expr, $default:expr) => {
            if let Some(handler) = VMEXIT_HANDLERS.read().get(&$handler) {
                handler(data, guest_regs)
            } else {
                $default(data, guest_regs)
            }
        };
    }

    let exit_type = match data.guest_vmcb.control_area.exit_code {
        VmExitCode::VMEXIT_RDTSC => call_handler!(VmExitType::Rdtsc),
        VmExitCode::VMEXIT_RDTSCP => call_handler!(VmExitType::Rdtscp),
        VmExitCode::VMEXIT_EXCEPTION_BP => call_handler!(VmExitType::Breakpoint),
        VmExitCode::VMEXIT_VMMCALL => call_handler!(VmExitType::Vmcall),
        VmExitCode::VMEXIT_NPF => call_handler!(VmExitType::NestedPageFault, npt::handle_default),
        VmExitCode::VMEXIT_CPUID => call_handler!(
            VmExitType::Cpuid(guest_regs.rax as u32 /* leaf */),
            cpuid::handle_default
        ),
        VmExitCode::VMEXIT_MSR => {
            let msr = guest_regs.rcx as u32;

            let map = VMEXIT_HANDLERS.read();
            if let Some(handler) = map.get(&VmExitType::Msr(msr)) {
                handler(data, guest_regs)
            } else if let Some(handler) = map.get(&VmExitType::Rdmsr(msr)) {
                handler(data, guest_regs)
            } else if let Some(handler) = map.get(&VmExitType::Wrmsr(msr)) {
                handler(data, guest_regs)
            } else {
                msr::handle_default(data, guest_regs)
            }
        }
        VmExitCode::VMEXIT_VMRUN => {
            EventInjection::gp().inject(data);
            ExitType::Continue
        }
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
        ExitType::ExitHypervisor => exit_hypervisor(data, guest_regs),
        ExitType::IncrementRIP => {
            // Reflect potentially updated guest's RAX to VMCB. Again, unlike other GPRs,
            // RAX is loaded from VMCB on VMRUN. Afterwards, advance RIP to "complete" the
            // instruction.
            //
            data.guest_vmcb.save_area.rax = guest_regs.rax;
            data.guest_vmcb.save_area.rip = data.guest_vmcb.control_area.nrip;
        }
        ExitType::Continue => {}
    }

    (exit_type == ExitType::ExitHypervisor) as u8
}
