use crate::nt::addresses::physical_address;
use crate::nt::include::Context;
use crate::nt::memory::{AlignedMemory, PAGE_SIZE};
use crate::svm::data::msr_bitmap::SVM_MSR_VM_HSAVE_PA;
use crate::svm::data::shared_data::SharedData;
use crate::svm::vmcb::control_area::{InterceptMisc1, InterceptMisc2};
use crate::{nt::include::KTRAP_FRAME, svm::vmcb::Vmcb};

use core::arch::asm;
use core::ops::{Deref, DerefMut};
use nt::include::PVOID;
use x86::controlregs::cr3;
use x86::msr::wrmsr;

pub const KERNEL_STACK_SIZE: usize = 0x6000;
pub const STACK_CONTENTS_SIZE: usize =
    KERNEL_STACK_SIZE - (core::mem::size_of::<PVOID>() * 6) - core::mem::size_of::<KTRAP_FRAME>();

#[repr(C)]
pub struct HostStackLayout {
    pub stack_contents: [u8; STACK_CONTENTS_SIZE],
    pub trap_frame: KTRAP_FRAME,

    /// HostRsp
    pub guest_vmcb_pa: u64,
    pub host_vmcb_pa: u64,

    pub self_data: *mut ProcessorData,
    pub shared_data: *const SharedData,

    /// To keep HostRsp 16 bytes aligned
    pub padding_1: u64,
    pub reserved_1: u64,
}

/// The data for a single **virtual** processor.
#[repr(C, align(4096))]
pub struct ProcessorData {
    /// Taken from SimpleSvm.
    ///
    /// ```
    ///  Low     HostStackLimit[0]                        StackLimit
    ///  ^       ...
    ///  ^       HostStackLimit[KERNEL_STACK_SIZE - 2]    StackBase
    ///  High    HostStackLimit[KERNEL_STACK_SIZE - 1]    StackBase
    /// ```
    ///
    pub host_stack_layout: HostStackLayout,
    pub guest_vmcb: Vmcb,
    pub host_vmcb: Vmcb,
    pub host_state_area: [u8; PAGE_SIZE],
}

impl ProcessorData {
    pub fn new() -> Option<AlignedMemory<Self>> {
        AlignedMemory::alloc(core::mem::size_of::<Self>())
    }
}

// TODO: Remove this wrapper and just use `ProcessorData::prepare(memory)` instead
pub struct ProcessorDataWrapper {
    pub data: AlignedMemory<ProcessorData>,
}

impl ProcessorDataWrapper {
    pub fn new() -> Option<Self> {
        ProcessorData::new().map(|data| ProcessorDataWrapper { data })
    }

    pub fn prepare_for_virtualization(&mut self, shared_data: &SharedData, context: Context) {
        // Based on this: https://github.com/tandasat/SimpleSvm/blob/master/SimpleSvm/SimpleSvm.cpp#L982

        // Get physical addresses of important data structures
        //
        let guest_vmcb_pa = physical_address(unsafe { &(**self.data).guest_vmcb as *const _ as _ });
        let host_vmcb_pa = physical_address(unsafe { &(**self.data).host_vmcb as *const _ as _ });
        let host_state_area_pa =
            physical_address(unsafe { (**self.data).host_state_area.as_ptr() as *const _ });
        let pml4_pa =
            physical_address(unsafe { (**shared_data.npt).pml4_entries.as_ptr() as *const _ });
        let msr_pm_pa = physical_address(shared_data.msr_permission_map.bitmap as *const _);

        log::info!("Physical addresses:");
        log::info!("guest_vmcb_pa: {:x}", guest_vmcb_pa);
        log::info!("guest_vmcb: {:x}", guest_vmcb_pa.as_u64());
        log::info!("host_vmcb_pa: {:x}", host_vmcb_pa);
        log::info!("host_vmcb: {:x}", host_vmcb_pa.as_u64());
        log::info!("host_state_area_pa: {:x}", host_state_area_pa);
        log::info!("pml4_pa: {:x}", pml4_pa);
        log::info!("msr_pm_pa: {:x}", msr_pm_pa);

        // Configure which instructions to intercept
        //
        log::info!("Configuring instructions to intercept");
        unsafe {
            (**self.data)
                .guest_vmcb
                .control_area
                .intercept_misc1
                .insert(InterceptMisc1::INTERCEPT_CPUID);

            (**self.data)
                .guest_vmcb
                .control_area
                .intercept_misc2
                .insert(InterceptMisc2::INTERCEPT_VMRUN);
        };

        // Trigger #VMEXIT on MSR exit as defined in msr permission map.
        //
        unsafe {
            (**self.data)
                .guest_vmcb
                .control_area
                .intercept_misc1
                .insert(InterceptMisc1::INTERCEPT_MSR_PROT);

            (**self.data).guest_vmcb.control_area.msrpm_base_pa = msr_pm_pa.as_u64();
        };

        // Specify guest's address space ID (ASID). TLB is maintained by the ID for
        // guests. Use the same value for all processors since all of them run a
        // single guest in our case. Use 1 as the most likely supported ASID by the
        // processor. The actual the supported number of ASID can be obtained with
        // CPUID. See "CPUID Fn8000_000A_EBX SVM Revision and Feature
        // Identification". Zero of ASID is reserved and illegal.
        //
        // See this for explanation of what an ASID is: https://stackoverflow.com/a/52725044
        //
        unsafe { (**self.data).guest_vmcb.control_area.guest_asid = 1 };

        // Enable nested page tables.
        //
        log::info!("Configuring nested page tables");
        unsafe {
            // (**self.data)
            //     .guest_vmcb
            //     .control_area
            //     .np_enable
            //     .insert(NpEnable::NESTED_PAGING);

            // (**self.data).guest_vmcb.control_area.ncr3 = pml4_pa.as_u64();
            (**self.data).guest_vmcb.control_area.ncr3 = cr3();
        };

        // Setup guest state based on current system state.
        //
        log::info!("Configuring guest state save area");
        unsafe { (**self.data).guest_vmcb.save_area.build(context) };

        // Save some of the current state on VMCB.
        //
        // See:
        // - https://docs.microsoft.com/en-us/cpp/intrinsics/svm-vmsave?view=msvc-170
        // - 15.5.2 VMSAVE and VMLOAD Instructions
        //
        log::info!("Saving current guest state on VMCB");
        unsafe { asm!("vmsave rax", in("rax") guest_vmcb_pa.as_u64()) };

        // Store data to stack so that the host (hypervisor) can use those values.
        //
        log::info!("Setting up the stack layout");
        unsafe {
            (**self.data).host_stack_layout.reserved_1 = u64::MAX;
            (**self.data).host_stack_layout.shared_data = shared_data as *const _;
            (**self.data).host_stack_layout.self_data = *self.data as _;
            (**self.data).host_stack_layout.host_vmcb_pa = host_vmcb_pa.as_u64();
            (**self.data).host_stack_layout.guest_vmcb_pa = guest_vmcb_pa.as_u64();
        }

        // Set the physical address for the `vmrun` instruction, which will save
        // the current host state.
        //

        log::info!("Setting the host state area in SVM_MSR_VM_HSAVE_PA");
        unsafe { wrmsr(SVM_MSR_VM_HSAVE_PA, host_state_area_pa.as_u64()) };

        // Also save current state for the host.
        //
        log::info!("Saving current host state on VMCB");
        unsafe { asm!("vmsave rax", in("rax") host_vmcb_pa.as_u64()) };
    }
}

impl Deref for ProcessorDataWrapper {
    type Target = AlignedMemory<ProcessorData>;

    fn deref(&self) -> &Self::Target {
        &self.data
    }
}

impl DerefMut for ProcessorDataWrapper {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.data
    }
}
