use crate::{
    svm::{
        shared_data::SharedData,
        support,
        utils::msr::EFER_SVME,
        vcpu_data::VcpuData,
        vmexit::{cpuid::CPUID_DEVIRTUALIZE},
        vmlaunch::launch_vm,
    },
    utils::{
        debug::dbg_break,
        nt::{Context, KeBugCheck, MANUALLY_INITIATED_CRASH},
    },
};
use alloc::{boxed::Box};
use core::lazy::OnceCell;
use x86::{
    cpuid::cpuid,
    msr::{rdmsr, wrmsr, IA32_EFER},
};

pub struct Vcpu {
    /// The index of the processor.
    index: u32,

    data: OnceCell<Box<VcpuData>>,
}

impl Vcpu {
    pub fn new(index: u32) -> Option<Self> {
        log::trace!("Creating processor {}", index);

        Some(Self {
            index,
            data: OnceCell::new(),
        })
    }

    pub fn virtualize(&mut self, shared_data: &mut SharedData) -> bool {
        log::info!("Virtualizing processor {}", self.index);

        // Based on this: https://github.com/tandasat/SimpleSvm/blob/master/SimpleSvm/SimpleSvm.cpp#L1137

        // IMPORTANT: We have to capture the context right here, so that `launch_vm`
        // continues the execution of the current process at this point of time.
        // If we don't do this, weird things will happen since we will execute
        // the guest at another point.
        //
        // This also makes sense why `vmsave` was crashing inside
        // `prepare_for_virtualization`. It obviously entered the guest state
        // and tried to execute from there on. And because of that, everything
        // that happened afterwards is just undefined behaviour.
        //
        // Literally wasted like a whole day just because of this 1 line.
        //
        log::info!("Capturing context");
        let context = Context::capture();

        // Check if already virtualized.
        //
        if !support::is_virtualized() {
            log::info!("Preparing for virtualization");

            support::set_virtualized();

            // Enable SVM by setting EFER.SVME.
            let msr = unsafe { rdmsr(IA32_EFER) } | EFER_SVME;
            unsafe { wrmsr(IA32_EFER, msr) };

            // Setup processor utils and get host rsp.
            //
            let host_rsp = &self
                .data
                .get_or_init(|| VcpuData::new(shared_data, context))
                .host_stack_layout
                .guest_vmcb_pa as *const u64 as *mut u64;

            // Launch vm
            // https://github.com/tandasat/SimpleSvm/blob/master/SimpleSvm/x64.asm#L78
            //
            log::info!("Launching vm");
            unsafe { launch_vm(host_rsp) };

            // We should never continue the guest execution here.
            //
            dbg_break!();
            unsafe { KeBugCheck(MANUALLY_INITIATED_CRASH) };
        }

        true
    }

    pub fn devirtualize(&self) -> bool {
        // Already devirtualized? Then we don't need to do anything.
        //
        let result = cpuid!(CPUID_DEVIRTUALIZE);
        if result.ecx != 0xDEADBEEF {
            log::info!(
                "Ecx is not 0xDEADBEEF. Nothing to do. Ecx: {:x}",
                result.ecx
            );
            return true;
        }

        log::info!("Processor {} has been devirtualized", self.index);

        true
    }

    pub fn id(&self) -> u32 {
        self.index
    }
}
