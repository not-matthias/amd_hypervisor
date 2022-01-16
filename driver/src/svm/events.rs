use crate::svm::data::processor::ProcessorData;
use bitfield::bitfield;

bitfield! {
    /// See `15.20 Event Injection`.
    ///
    /// The VMM can inject exceptions or interrupts (collectively referred to as events) into the guest by
    /// setting bits in the VMCB’s EVENTINJ field prior to executing the VMRUN instruction
    ///
    /// When an event is injected by means of this mechanism, the VMRUN instruction causes the guest to take the
    /// specified exception or interrupt unconditionally before executing the first guest instruction.
    ///
    /// ## Fields
    ///
    /// - `Vector`: The 8-bit IDT vector of the interrupt or exception. If `TYPE` is 2 (NMI), the `VECTOR` field is ignored.
    /// - `Type`: Qualifies the guest exception or interrupt to generate. The following values are defined:
    ///     - 0: External or virtual interrupt (INTR)
    ///     - 2: NMI
    ///     - 3: Exception (fault or trap)
    ///     - 4: Software interrupt (INTn instruction)
    /// - `Error Code Valid` (`EV`): Set to 1 if the exception should push an error code onto the stack; clear to 0 otherwise.
    /// - `Valid`: Set to 1 if an event is to be inject into the guest; clear to 0 otherwise.
    /// - `Error Code`: If `EV` is set to 1, the error code to be pushed onto the stack, ignored otherwise.
    ///
    pub struct EventInjection(u64);
    impl Debug;
    pub get_vector, set_vector: 7, 0;                       // [0-7]
    pub get_type, set_type: 10, 8;                          // [8-10]
    pub get_error_code_valid, set_error_code_valid: 11, 11; // [11]
    // Reserved                                             // [12-30]
    pub get_valid, set_valid: 31, 31;                       // [31]
    pub get_error_code, set_error_code: 63, 32;             // [32-63]
}

impl EventInjection {
    /// See `8 Exceptions and Interrupts > 8.2 Vectors > 8.2.14 #GP`.
    pub fn gp() -> Self {
        let mut event = EventInjection(0);
        event.set_vector(13); // #GP
        event.set_type(3); // Exception
        event.set_error_code_valid(1);
        event.set_valid(1);

        event
    }

    pub fn bp() -> Self {
        let mut event = EventInjection(0);
        event.set_vector(3); // #BP
        event.set_type(3); // Exception
        event.set_valid(1);

        event
    }

    /// Injects the current event into the guest vmcb.
    pub fn inject(&self, data: &mut ProcessorData) {
        data.guest_vmcb.control_area.event_inj = self.0;
    }
}
