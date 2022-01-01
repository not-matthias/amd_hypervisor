#![no_std]

// use amd_hypervisor::nt::include::KTRAP_FRAME;

#[test]
fn test_struct_sizes() {
    // Currently not working.
    // TODO: Implement

    // assert_eq!(std::mem::size_of::<KTRAP_FRAME>(), 0x190);
    // assert_eq!(
    //     std::mem::size_of::<ProcessorData>(),
    //     KERNEL_STACK_SIZE + PAGE_SIZE * 3
    // );
    // assert_eq!(std::mem::size_of::<SaveArea>(), 0x298);
    // assert_eq!(std::mem::size_of::<Vmcb>(), PAGE_SIZE);
    // assert_eq!(std::mem::size_of::<ControlArea>(), 0x400);
    // assert_eq!(std::mem::size_of::<GuestRegisters>(), 128);
    // assert_eq!(std::mem::size_of::<HostStackLayout>(), 0x6000);
}
