use crate::nt::include::RtlCopyMemory;
use crate::nt::memory::AllocatedMemory;

pub const JMP_SHELLCODE_LEN: usize = 15;

pub struct InlineHook {
    trampoline: AllocatedMemory<u8>,
    jmp_to_handler: [u8; JMP_SHELLCODE_LEN],

    address: u64,
    handler: u64,

    enabled: bool,
}

impl InlineHook {
    // Note: We have to allocate a new instance here, so that it's valid after the virtualization. Otherwise,
    // all the addresses would be 0x0.
    pub fn new(address: u64, handler: *const ()) -> Option<AllocatedMemory<Self>> {
        log::info!("Creating a new inline hook");

        let mut hook = AllocatedMemory::<Self>::alloc(core::mem::size_of::<Self>())?;
        hook.trampoline = Self::trampoline(address as u64)?;
        hook.jmp_to_handler = Self::jmp_shellcode(handler as u64);
        hook.enabled = false;
        hook.address = address;
        hook.handler = handler as u64;

        Some(hook)
    }

    pub fn enable(&self) {
        log::info!("Enabling inline hook");

        let jmp_to_handler = Self::jmp_shellcode(self.handler);

        log::info!("jmp_to_handler: {:x?}", self.jmp_to_handler);
        log::info!(
            "Writing the shellcode {:x?} to {:#x} (size = {:?}).",
            self.jmp_to_handler,
            self.handler as u64,
            self.jmp_to_handler.len()
        );

        // Note: In order for this to work, we have to use an heap allocated instance instead of
        // a stack allocated one. Otherwise, the stack will be invalidated after the virtualization of
        // the current processor. After that, all the variables will be set to 0.
        //
        unsafe {
            RtlCopyMemory(
                self.address as *mut u64,
                jmp_to_handler.as_ptr() as _,
                JMP_SHELLCODE_LEN,
            );
        }
    }

    pub fn trampoline_address(&self) -> *mut u64 {
        self.trampoline.as_ptr() as _
    }

    fn jmp_shellcode(target_address: u64) -> [u8; JMP_SHELLCODE_LEN] {
        log::info!(
            "Creating the jmp shellcode for address: {:#x}",
            target_address
        );

        let shellcode: [u8; 15] = [
            0x90, // nop
            0xff, 0x25, 0x00, 0x00, 0x00, 0x00, // jmp qword ptr [rip]
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // address
        ];
        unsafe { (shellcode.as_ptr().offset(7) as *mut u64).write_volatile(target_address) };

        shellcode
    }

    /// Creates the trampoline that executes the original code and continues right after the inline hook.
    ///
    /// The trampoline looks like this:
    /// <start bytes overwritten by jmp>
    /// <jmp to original function>
    ///
    fn trampoline(address: u64) -> Option<AllocatedMemory<u8>> {
        log::info!("Creating the trampoline for address: {:#x}", address);

        let stub = AllocatedMemory::<u8>::alloc(JMP_SHELLCODE_LEN * 2)?;

        // Copy the original bytes
        //
        unsafe { RtlCopyMemory(stub.as_ptr() as _, address as _, JMP_SHELLCODE_LEN) };

        // Copy the jump
        //
        let jmp_back = Self::jmp_shellcode(address + JMP_SHELLCODE_LEN as u64); // Jump right after `jmp_to_handler`
        unsafe {
            RtlCopyMemory(
                stub.as_ptr().add(JMP_SHELLCODE_LEN) as _,
                jmp_back.as_ptr() as _,
                JMP_SHELLCODE_LEN,
            )
        };

        Some(stub)
    }
}
