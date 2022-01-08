use crate::nt::include::RtlCopyMemory;
use crate::nt::memory::AllocatedMemory;
use alloc::vec::Vec;
use core::arch::x86_64::_mm_clflush;
use iced_x86::{
    BlockEncoder, BlockEncoderOptions, Code, Decoder, DecoderOptions, FlowControl, Instruction,
    InstructionBlock, Register,
};
use snafu::prelude::*;

pub const JMP_SHELLCODE_LEN: usize = 14;

#[derive(Debug, Snafu)]
pub enum InlineHookError {
    #[snafu(display("Failed to parse bytes of original function"))]
    InvalidBytes,

    #[snafu(display("Couldn't find enough space for the jump shellcode"))]
    NotEnoughBytes,

    #[snafu(display("Failed to find original instructions"))]
    NoInstructions,

    #[snafu(display("Failed to allocate memory for trampoline"))]
    AllocationFailed,

    #[snafu(display("Failed to encode trampoline"))]
    EncodingFailed,
}

pub struct InlineHook {
    trampoline: AllocatedMemory<u8>,

    address: u64,
    handler: u64,

    enabled: bool,
}

impl InlineHook {
    /// Creates a new inline hook (not yet enabled) for the specified function.
    ///
    ///
    /// ## Note
    ///
    /// Note: We have to allocate a new instance here, so that it's valid after the virtualization. Otherwise,
    /// all the addresses would be 0x0.
    ///
    /// TODO: Can we somehow get rid of the original address?
    pub fn new(
        original_address: u64,
        address: u64,
        handler: *const (),
    ) -> Option<AllocatedMemory<Self>> {
        log::info!(
            "Creating a new inline hook. Address: {:x}, handler: {:x}",
            address,
            handler as u64
        );

        let mut hook = AllocatedMemory::<Self>::alloc(core::mem::size_of::<Self>())?;
        hook.trampoline = match Self::trampoline_shellcode(original_address, address as u64) {
            Ok(trampoline) => trampoline,
            Err(e) => {
                log::error!("Failed to create trampoline: {:?}", e);
                return None;
            }
        };
        hook.enabled = false;
        hook.address = address;
        hook.handler = handler as u64;

        Some(hook)
    }

    pub fn enable(&self) {
        let jmp_to_handler = Self::jmp_shellcode(self.handler);
        log::info!(
            "Writing the shellcode {:x?} to {:p} (size = {:?}).",
            jmp_to_handler,
            self.trampoline_address(),
            jmp_to_handler.len()
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

    /// Creates the jmp shellcode.
    ///
    /// ## How it works.
    ///
    /// We are using the following assembly shellcode:
    /// ```asm
    /// jmp [rip+00h]
    /// 0xDEADBEEF
    /// ```
    ///
    /// Or in a different format:
    ///
    /// ```asm
    /// jmp qword ptr cs:jmp_add
    /// jmp_addr: dq 0xDEADBEEF
    /// ```
    ///
    /// The core premise behind it is, that we jump to the address that is right after the current
    /// instruction.  
    ///
    /// ## Why use this instead of `mov rax, jmp rax`?
    ///
    /// This shellcode has one very important feature: **It doesn't require any registers to store the
    /// jmp address**. And because of that, we don't have to fear overwriting some register values.
    ///
    fn jmp_shellcode(target_address: u64) -> [u8; 14] {
        log::info!(
            "Creating the jmp shellcode for address: {:#x}",
            target_address
        );

        // Create the shellcode. See function documentation for more information.
        //
        let mut shellcode = [
            0xff, 0x25, 0x00, 0x00, 0x00, 0x00, 0xCC, 0xCC, 0xCC, 0xCC, 0xCC, 0xCC, 0xCC, 0xCC,
        ];
        unsafe {
            (shellcode.as_mut_ptr().add(6) as *mut u64).write_volatile(target_address as u64)
        };

        shellcode
    }

    /// Creates a trampoline shellcode that jumps to the original function.
    ///
    /// ## Parameters
    ///
    /// - `original_address`: The address of the original function. We need this so that we can relocate potential jumps that have been overwritten by the hook.
    /// - `function_address`: The address of same function in the copied page.
    ///
    /// TODO: Replace one of these parameters
    fn trampoline_shellcode(
        original_address: u64,
        function_address: u64,
    ) -> Result<AllocatedMemory<u8>, InlineHookError> {
        log::info!(
            "Creating the trampoline shellcode for function: {:#x}",
            function_address
        );

        // Read bytes from function and decode them. Read 2 times the amount needed, in case there are
        // bigger instructions that take more space.
        //
        let bytes = unsafe {
            core::slice::from_raw_parts(function_address as *mut u8, JMP_SHELLCODE_LEN * 2)
        };
        let mut decoder = Decoder::with_ip(64, &bytes, original_address, DecoderOptions::NONE);

        let mut total_bytes = 0;
        let mut trampoline = Vec::new();
        for instr in &mut decoder {
            if instr.is_invalid() {
                return Err(InlineHookError::InvalidBytes);
            }

            // Create the new trampoline instruction
            //
            match instr.flow_control() {
                FlowControl::Next | FlowControl::Return => {
                    total_bytes += instr.len();
                    trampoline.push(instr);
                }
                FlowControl::Call => {
                    if instr.is_call_near() {
                        total_bytes += instr.len();

                        let branch_target = instr.near_branch_target();

                        // TODO: Just relocate the relative jump

                        // mov rax, branch_target
                        // jmp rax
                        //
                        // let mov_rax =
                        //     Instruction::with2(Code::Mov_r64_imm64, Register::RAX, branch_target)
                        //         .unwrap();
                        // let jmp_rax = Instruction::with1(Code::Jmp_rm64, Register::RAX).unwrap();
                        //
                        // trampoline.push(mov_rax);
                        // trampoline.push(jmp_rax);
                        trampoline.push(
                            Instruction::with_branch(Code::Jmp_rel32_64, branch_target).unwrap(),
                        );
                    } else {
                        log::warn!("Found call far");
                    }
                }
                FlowControl::IndirectBranch
                | FlowControl::ConditionalBranch
                | FlowControl::UnconditionalBranch
                | FlowControl::IndirectCall
                | FlowControl::Interrupt
                | FlowControl::XbeginXabortXend
                | FlowControl::Exception => log::warn!("Unsupported instruction"),
            };

            if total_bytes >= JMP_SHELLCODE_LEN {
                break;
            }
        }

        if total_bytes < JMP_SHELLCODE_LEN {
            return Err(InlineHookError::NotEnoughBytes);
        }

        if trampoline.is_empty() {
            return Err(InlineHookError::NoInstructions);
        }

        // Create a jmp instruction at the end of the trampoline, back to the original function. We
        // don't need to do that, if there's already a return instruction.
        //
        let last_instr = trampoline.last().unwrap();
        let jmp_back_address = last_instr.next_ip();
        if last_instr.flow_control() != FlowControl::Return {
            log::info!(
                "Creating jmp back to original instructions at {:#x}",
                jmp_back_address
            );
            trampoline
                .push(Instruction::with_branch(Code::Jmp_rel32_64, jmp_back_address).unwrap());
        }

        // Allocate new memory for the trampoline and encode the instructions.
        //
        let memory = AllocatedMemory::<u8>::alloc(total_bytes)
            .ok_or_else(|| InlineHookError::AllocationFailed)?;

        log::info!("Allocated trampoline memory at {:p}", memory.as_ptr());
        log::info!(
            "Offset between original and trampoline: {:#x}",
            function_address.abs_diff(memory.as_ptr() as u64)
        );

        let block = InstructionBlock::new(&trampoline, memory.as_ptr() as _);
        let encoded = BlockEncoder::encode(decoder.bitness(), block, BlockEncoderOptions::NONE)
            .map(|b| b.code_buffer)
            .map_err(|_| InlineHookError::EncodingFailed)?;

        log::info!("Encoded trampoline: {:x?}", encoded);

        // Copy the encoded bytes and return the allocated memory.
        //
        unsafe { core::ptr::copy_nonoverlapping(encoded.as_ptr(), memory.as_ptr(), encoded.len()) };

        Ok(memory)
    }
}
