use crate::{
    svm::{
        data::shared_data::SharedData,
        msr::SVM_MSR_VM_HSAVE_PA,
        vmcb::{
            control_area::{ExceptionVector, InterceptMisc1, InterceptMisc2, NpEnable},
            Vmcb,
        },
        vmexit::vmexit_installed,
        VmExitType,
    },
    utils::{
        addresses::physical_address,
        nt::{Context, KTRAP_FRAME},
    },
};
use alloc::boxed::Box;
use core::{any::Any, arch::asm, ptr, ptr::NonNull};
use x86::{
    bits64::paging::{PAddr, BASE_PAGE_SIZE},
    controlregs::cr3,
    msr::wrmsr,
};

pub const KERNEL_STACK_SIZE: usize = 0x6000;
pub const STACK_CONTENTS_SIZE: usize = KERNEL_STACK_SIZE
    - (core::mem::size_of::<*mut u64>() * 6)
    - core::mem::size_of::<KTRAP_FRAME>();

#[repr(C, align(4096))]
pub struct HostStackLayout {
    pub stack_contents: [u8; STACK_CONTENTS_SIZE],
    pub trap_frame: KTRAP_FRAME,

    /// HostRsp
    pub guest_vmcb_pa: u64,
    pub host_vmcb_pa: u64,

    pub self_data: *mut u64, // Needed for the `vmlaunch` assembly function
    pub shared_data: NonNull<SharedData>,

    // To keep HostRsp 16 bytes aligned
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
    pub(crate) host_stack_layout: HostStackLayout,
    pub guest_vmcb: Vmcb,
    pub host_vmcb: Vmcb,
    pub(crate) host_state_area: [u8; BASE_PAGE_SIZE],
}
const_assert_eq!(
    core::mem::size_of::<ProcessorData>(),
    KERNEL_STACK_SIZE + 3 * BASE_PAGE_SIZE
);

impl ProcessorData {
    pub(crate) fn new(shared_data: &mut SharedData, context: Context) -> Box<Self> {
        // Create instance
        //
        let instance = Self {
            host_stack_layout: HostStackLayout {
                stack_contents: [0u8; STACK_CONTENTS_SIZE],
                trap_frame: unsafe { core::mem::zeroed() },
                guest_vmcb_pa: 0,
                host_vmcb_pa: 0,
                self_data: ptr::null_mut(), // We set this later.
                shared_data: unsafe { NonNull::new_unchecked(shared_data as *mut _) },
                padding_1: u64::MAX,
                reserved_1: u64::MAX,
            },
            guest_vmcb: unsafe { core::mem::zeroed() },
            host_vmcb: unsafe { core::mem::zeroed() },
            host_state_area: [0u8; BASE_PAGE_SIZE],
        };
        let mut instance = Box::new(instance);

        instance.host_stack_layout.self_data = &mut *instance as *mut _ as _;
        instance.host_stack_layout.host_vmcb_pa =
            physical_address(&instance.host_vmcb as *const _ as _).as_u64();
        instance.host_stack_layout.guest_vmcb_pa =
            physical_address(&instance.guest_vmcb as *const _ as _).as_u64();

        // Get physical addresses of important data structures
        //
        let pml4_pa = physical_address(shared_data.primary_npt.pml4.as_ptr() as _);
        let msr_pm_pa = physical_address(shared_data.msr_bitmap.as_mut() as *mut _ as _);

        instance.configure_interceptions();
        instance.configure_npt(pml4_pa);
        instance.configure_msr_bitmap(msr_pm_pa);
        instance.configure_vmcb();

        // Setup guest state based on current system state.
        //
        instance.guest_vmcb.save_area.build(context);

        instance
    }

    fn configure_vmcb(&mut self) {
        // Save some of the current state on VMCB.
        //
        // See:
        // - https://docs.microsoft.com/en-us/cpp/intrinsics/svm-vmsave?view=msvc-170
        // - 15.5.2 VMSAVE and VMLOAD Instructions
        //

        log::info!("Saving current guest state on VMCB");
        unsafe { asm!("vmsave rax", in("rax") self.host_stack_layout.guest_vmcb_pa) };

        // Set the physical address for the `vmrun` instruction, which will save
        // the current host state.
        //
        log::info!("Setting the host state area in SVM_MSR_VM_HSAVE_PA");
        let host_state_area_pa = physical_address(self.host_state_area.as_ptr() as *const _);
        unsafe { wrmsr(SVM_MSR_VM_HSAVE_PA, host_state_area_pa.as_u64()) };

        // Also save current state for the host.
        //
        log::info!("Saving current host state on VMCB");
        unsafe { asm!("vmsave rax", in("rax") self.host_stack_layout.host_vmcb_pa) };
    }

    fn configure_interceptions(&mut self) {
        // TODO: Allow custom exceptions to be hooked. This is tricky, because we can't
        // map ExceptionVector to VmExitCode.

        if vmexit_installed!(VmExitType::Breakpoint) {
            log::info!("Intercepting breakpoint");
            self.guest_vmcb
                .control_area
                .intercept_exception
                .insert(ExceptionVector::BREAKPOINT);
        }

        if vmexit_installed!(VmExitType::Cpuid(_)) {
            log::info!("Intercepting cpuid");
            self.guest_vmcb
                .control_area
                .intercept_misc1
                .insert(InterceptMisc1::INTERCEPT_CPUID);
        }

        if vmexit_installed!(VmExitType::Rdtsc) {
            log::info!("Intercepting rdtsc");
            self.guest_vmcb
                .control_area
                .intercept_misc1
                .insert(InterceptMisc1::INTERCEPT_RDTSC);
        }

        if vmexit_installed!(VmExitType::Rdtscp) {
            log::info!("Intercepting rdtscp");
            self.guest_vmcb
                .control_area
                .intercept_misc2
                .insert(InterceptMisc2::INTERCEPT_RDTSCP);
        }

        if vmexit_installed!(VmExitType::Vmcall) {
            log::info!("Intercepting vmcall");
            self.guest_vmcb
                .control_area
                .intercept_misc2
                .insert(InterceptMisc2::INTERCEPT_VMCALL);
        }

        self.guest_vmcb
            .control_area
            .intercept_misc2
            .insert(InterceptMisc2::INTERCEPT_VMRUN);
    }

    fn configure_npt(&mut self, pml4_pa: PAddr) {
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

        // Enable nested page tables (only when we also specify a page fault handler).
        //
        if vmexit_installed!(VmExitType::NestedPageFault) {
            log::info!("Configuring nested page tables");
            self.guest_vmcb
                .control_area
                .np_enable
                .insert(NpEnable::NESTED_PAGING);

            self.guest_vmcb.control_area.ncr3 = pml4_pa.as_u64();
            log::info!("Pml4 pa: {:x}", pml4_pa.as_u64());
        } else {
            log::info!("Using system page tables");
            self.guest_vmcb.control_area.ncr3 = unsafe { cr3() };
        }
    }

    /// Trigger #VMEXIT on MSR exit as defined in msr permission map.
    fn configure_msr_bitmap(&mut self, msr_pa: PAddr) {
        self.guest_vmcb
            .control_area
            .intercept_misc1
            .insert(InterceptMisc1::INTERCEPT_MSR_PROT);

        self.guest_vmcb.control_area.msrpm_base_pa = msr_pa.as_u64();
    }
}

// Helper functions to make life a little easier.
impl ProcessorData {
    pub fn shared_data(&mut self) -> &mut SharedData {
        unsafe { self.host_stack_layout.shared_data.as_mut() }
    }

    pub fn custom_data<T: 'static + Any>(&mut self) -> Option<&mut T> {
        None
        // self.custom_data.downcast_mut()
    }
}
