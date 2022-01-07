use crate::dbg_break;
use crate::nt::include::RtlCopyMemory;
use crate::nt::memory::AllocatedMemory;
use alloc::vec::Vec;
use iced_x86::{
    BlockEncoder, BlockEncoderOptions, Code, Decoder, DecoderOptions, FlowControl, Instruction,
    InstructionBlock, Register,
};
use snafu::prelude::*;

pub const JMP_SHELLCODE_LEN: usize = 12;

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
    // Note: We have to allocate a new instance here, so that it's valid after the virtualization. Otherwise,
    // all the addresses would be 0x0.
    pub fn new(address: u64, handler: *const ()) -> Option<AllocatedMemory<Self>> {
        log::info!(
            "Creating a new inline hook. Address: {:x}, handler: {:x}",
            address,
            handler as u64
        );

        let mut hook = AllocatedMemory::<Self>::alloc(core::mem::size_of::<Self>())?;
        hook.trampoline = match Self::trampoline_shellcode(address as u64) {
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

    fn jmp_shellcode(target_address: u64) -> Vec<u8> {
        log::info!(
            "Creating the jmp shellcode for address: {:#x}",
            target_address
        );

        // Create the instructions:
        //
        // nop
        // mov rax, target_address
        // jmp rax
        //
        let instructions = [
            Instruction::with(Code::Nopq),
            Instruction::with2(Code::Mov_r64_imm64, Register::RAX, target_address).unwrap(),
            Instruction::with1(Code::Jmp_rm64, Register::RAX).unwrap(),
        ];

        // Encode the instructions. It's a absolute jump, so we don't have to care about the rip.
        //
        let block = InstructionBlock::new(&instructions, 0x0);
        let shellcode = BlockEncoder::encode(64, block, BlockEncoderOptions::NONE)
            .map(|b| b.code_buffer)
            .unwrap();

        shellcode
    }

    fn trampoline_shellcode(function_address: u64) -> Result<AllocatedMemory<u8>, InlineHookError> {
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
        let mut decoder = Decoder::with_ip(64, &bytes, function_address, DecoderOptions::NONE);

        let mut total_bytes = 0;
        let mut trampoline = Vec::new();
        for instr in &mut decoder {
            if instr.is_invalid() {
                return Err(InlineHookError::InvalidBytes);
            }

            // Create the new trampoline instruction
            //
            let instr = match instr.flow_control() {
                FlowControl::Next => instr,
                FlowControl::Call => {
                    // TODO: Relocate call
                    instr
                }
                FlowControl::Return => instr,
                FlowControl::IndirectBranch
                | FlowControl::ConditionalBranch
                | FlowControl::UnconditionalBranch
                | FlowControl::IndirectCall
                | FlowControl::Interrupt
                | FlowControl::XbeginXabortXend
                | FlowControl::Exception => {
                    log::warn!("Using unsupported instruction: {:?}", instr);
                    instr
                }
            };

            total_bytes += instr.len();
            trampoline.push(instr);

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
            trampoline
                .push(Instruction::with_branch(Code::Jmp_rel32_64, jmp_back_address).unwrap());
        }

        // Allocate new memory for the trampoline and encode the instructions.
        //
        let memory = AllocatedMemory::<u8>::alloc(total_bytes)
            .ok_or_else(|| InlineHookError::AllocationFailed)?;

        log::info!("Allocated trampoline memory at {:p}", memory.as_ptr());

        let block = InstructionBlock::new(&trampoline, memory.as_ptr() as _);
        let encoded = BlockEncoder::encode(decoder.bitness(), block, BlockEncoderOptions::NONE)
            .map(|b| b.code_buffer)
            .map_err(|_| InlineHookError::EncodingFailed)?;

        // Copy the encoded bytes and return the allocated memory.
        //
        unsafe { RtlCopyMemory(memory.as_ptr() as _, encoded.as_ptr() as _, encoded.len()) };

        dbg_break!();

        Ok(memory)
    }
}
