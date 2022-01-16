use crate::{
    svm::{
        data::{msr_bitmap::SVM_MSR_VM_HSAVE_PA, shared_data::SharedData},
        msr,
        vmcb::{
            control_area::{ExceptionVector, InterceptMisc1, InterceptMisc2, NpEnable},
            Vmcb,
        },
    },
    utils::{
        addresses::physical_address,
        memory::AllocatedMemory,
        nt::{Context, KTRAP_FRAME},
        ptr::Pointer,
    },
};
use core::{arch::asm, ptr::NonNull};
use nt::include::PVOID;
use x86::{bits64::paging::BASE_PAGE_SIZE, msr::wrmsr};

pub const KERNEL_STACK_SIZE: usize = 0x6000;
pub const STACK_CONTENTS_SIZE: usize =
    KERNEL_STACK_SIZE - (core::mem::size_of::<PVOID>() * 6) - core::mem::size_of::<KTRAP_FRAME>();

#[repr(C, align(4096))]
pub struct HostStackLayout {
    pub stack_contents: [u8; STACK_CONTENTS_SIZE],
    pub trap_frame: KTRAP_FRAME,

    /// HostRsp
    pub guest_vmcb_pa: u64,
    pub host_vmcb_pa: u64,

    pub self_data: Pointer<ProcessorData>,
    pub shared_data: Pointer<SharedData>,

    /// To keep HostRsp 16 bytes aligned
    pub padding_1: u64,
    pub reserved_1: u64,
}
const_assert_eq!(core::mem::size_of::<HostStackLayout>(), KERNEL_STACK_SIZE);

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
    pub host_stack_layout: HostStackLayout,
    pub guest_vmcb: Vmcb,
    pub host_vmcb: Vmcb,
    pub host_state_area: [u8; BASE_PAGE_SIZE],
}
const_assert_eq!(
    core::mem::size_of::<ProcessorData>(),
    KERNEL_STACK_SIZE + 3 * BASE_PAGE_SIZE
);

impl ProcessorData {
    pub fn new() -> Option<AllocatedMemory<Self>> {
        AllocatedMemory::alloc_aligned(core::mem::size_of::<Self>())
    }

    pub fn prepare_for_virtualization(&mut self, shared_data: &mut SharedData, context: Context) {
        // Based on this: https://github.com/tandasat/SimpleSvm/blob/master/SimpleSvm/SimpleSvm.cpp#L982

        // Get physical addresses of important data structures
        //
        let guest_vmcb_pa = physical_address(&self.guest_vmcb as *const _ as _);
        let host_vmcb_pa = physical_address(&self.host_vmcb as *const _ as _);
        let host_state_area_pa = physical_address(self.host_state_area.as_ptr() as *const _);
        let pml4_pa = physical_address(
            shared_data.hooked_npt.as_mut().rwx_npt.pml4.as_ptr() as *const _ as _
        );
        let msr_pm_pa = physical_address(shared_data.msr_permission_map.as_ptr() as *const _);

        log::trace!("Physical addresses:");
        log::trace!("guest_vmcb_pa: {:x}", guest_vmcb_pa);
        log::trace!("guest_vmcb: {:x}", guest_vmcb_pa.as_u64());
        log::trace!("host_vmcb_pa: {:x}", host_vmcb_pa);
        log::trace!("host_vmcb: {:x}", host_vmcb_pa.as_u64());
        log::trace!("host_state_area_pa: {:x}", host_state_area_pa);
        log::trace!("pml4_pa: {:x}", pml4_pa);
        log::trace!("msr_pm_pa: {:x}", msr_pm_pa);

        // Intercept breakpoint exceptions. This is required for the npt hooks because
        // we need to redirect the execution to our hook handlers. The breakpoint will
        // be placed on the original instruction.
        //
        self.guest_vmcb
            .control_area
            .intercept_exception
            .insert(ExceptionVector::BREAKPOINT);

        // Configure which instructions to intercept
        //
        log::info!("Configuring instructions to intercept");
        self.guest_vmcb
            .control_area
            .intercept_misc1
            .insert(InterceptMisc1::INTERCEPT_CPUID);

        self.guest_vmcb
            .control_area
            .intercept_misc2
            .insert(InterceptMisc2::INTERCEPT_VMRUN);

        // Trigger #VMEXIT on MSR exit as defined in msr permission map.
        //
        self.guest_vmcb
            .control_area
            .intercept_misc1
            .insert(InterceptMisc1::INTERCEPT_MSR_PROT);

        self.guest_vmcb.control_area.msrpm_base_pa = msr_pm_pa.as_u64();

        // Specify guest's address space ID (ASID). TLB is maintained by the ID for
        // guests. Use the same value for all processors since all of them run a
        // single guest in our case. Use 1 as the most likely supported ASID by the
        // processor. The actual the supported number of ASID can be obtained with
        // CPUID. See "CPUID Fn8000_000A_EBX SVM Revision and Feature
        // Identification". Zero of ASID is reserved and illegal.
        //
        // See this for explanation of what an ASID is: https://stackoverflow.com/a/52725044
        //
        self.guest_vmcb.control_area.guest_asid = 1;

        // Enable nested page tables.
        //
        log::info!("Configuring nested page tables");
        self.guest_vmcb
            .control_area
            .np_enable
            .insert(NpEnable::NESTED_PAGING);

        self.guest_vmcb.control_area.ncr3 = pml4_pa.as_u64();
        log::info!("Pml4 pa: {:x}", pml4_pa.as_u64());

        // Setup guest state based on current system state.
        //
        log::info!("Configuring guest state save area");
        self.guest_vmcb.save_area.build(context);

        // Save some of the current state on VMCB.
        //
        // See:
        // - https://docs.microsoft.com/en-us/cpp/intrinsics/svm-vmsave?view=msvc-170
        // - 15.5.2 VMSAVE and VMLOAD Instructions
        //
        log::info!("Saving current guest state on VMCB");
        unsafe { asm!("vmsave rax", in("rax") guest_vmcb_pa.as_u64()) };

        // Set the physical address for the `vmrun` instruction, which will save
        // the current host state.
        //
        log::info!("Setting the host state area in SVM_MSR_VM_HSAVE_PA");
        unsafe { wrmsr(SVM_MSR_VM_HSAVE_PA, host_state_area_pa.as_u64()) };

        // Also save current state for the host.
        //
        log::info!("Saving current host state on VMCB");
        unsafe { asm!("vmsave rax", in("rax") host_vmcb_pa.as_u64()) };

        // Store data to stack so that the host (hypervisor) can use those values.
        //
        log::info!("Setting up the stack layout");
        self.host_stack_layout.reserved_1 = u64::MAX;
        self.host_stack_layout.shared_data =
            Pointer(unsafe { NonNull::new_unchecked(shared_data as *mut _) });
        self.host_stack_layout.self_data =
            Pointer(unsafe { NonNull::new_unchecked(self as *mut _) });
        self.host_stack_layout.host_vmcb_pa = host_vmcb_pa.as_u64();
        self.host_stack_layout.guest_vmcb_pa = guest_vmcb_pa.as_u64();
    }
}
