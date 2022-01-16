.global launch_vm

// KTRAP_FRAME_SIZE: 0x190

launch_vm:
    // rsp = host_rsp
    //
    mov rsp, rcx

guest_loop:

    // Run the loop to execute the guest and handle #VMEXITs
    //
    // ----
    // Rsp          => 0x...fd0 GuestVmcbPa       ; HostStackLayout
    //                 0x...fd8 HostVmcbPa        ;
    //                 0x...fe0 Self              ;
    //                 0x...fe8 SharedVpData      ;
    //                 0x...ff0 Padding1          ;
    //                 0x...ff8 Reserved1         ;
    // ----
    //
    mov rax, [rsp]          // rax = processor_data.host_stack_layout.guest_vmcb_pa
    vmload rax              // load previous saved guest state from vmcb

    // Start the guest execution via the VMRUN instruction.
    //
    // This instruction does the following:
    // - Saves current state to the host state-save area defined in IA32_MSR_VM_HSAVE_PA.
    // - Loads guest state from VMCB state-save area.
    // - Enables interrupts by setting the global interrupt flag (GIF)
    // - Resumes execution of the guest until #VMEXIT occurs.
    //
    // For more information see: `15.5. VMRUN Instruction > 15.5.1 Basic Operation`.
    //
    // There's a few assumptions here:
    // - rax must be a 4-Kbyte aligned address.
    // - `VM_HSAVE_PA MSR` must be set so that the host processor state information can be saved.
    //
    vmrun rax               // switch to guest until #VMEXIT

    // When we get here, an #VMEXIT occurred. Some of the guest state has
    // been saved, but not all of it. Use the VMSAVE instruction to save
    // the rest to the VMCB.
    //
    // Rax (and other states like RSP) habe been restored from the host
    // state-save, so it has the value of the host and not the one of the
    // guest.
    //
    vmsave rax              // save current guest state to vmcb

    // Allocate trap frame so that WinDbg can display the strack trace of
    // the guest while handle_vmexit is being executed.
    sub rsp, 0x190          // KTRAP_FRAME_SIZE

    // Sve the general purpose registers of the guest since they are not
    // saved on #VMEXIT.
    //
    push    rax
    push    rcx
    push    rdx
    push    rbx
    push    -1      // Dummy for rsp.
    push    rbp
    push    rsi
    push    rdi
    push    r8
    push    r9
    push    r10
    push    r11
    push    r12
    push    r13
    push    r14
    push    r15

    // Set parameters for `handle_vmexit`.
    //
    // ----
    // Rsp                             => 0x...dc0 R15               ; GUEST_REGISTERS
    //                                    0x...dc8 R14               ;
    //                                             ...               ;
    //                                    0x...e38 RAX               ;
    // Rsp + 8 * 16                    => 0x...e40 TrapFrame         ; HostStackLayout
    //                                             ...               ;
    // Rsp + 8 * 16 + KTRAP_FRAME_SIZE => 0x...fd0 GuestVmcbPa       ;
    //                                    0x...fd8 HostVmcbPa        ;
    // Rsp + 8 * 18 + KTRAP_FRAME_SIZE => 0x...fe0 Self              ;
    //                                    0x...fe8 SharedVpData      ;
    //                                    0x...ff0 Padding1          ;
    //                                    0x...ff8 Reserved1         ;
    // ----
    //
    // Note: KTRAP_FRAME_SIZE is just 0 in our case since we didn't
    //       allocate it.
    //
    mov rdx, rsp                    // rdx = guest_registers
    mov rcx, [rsp + 8 * 18 + 0x190] // rcx = processor_data

    // Allocate stack for homing space (0x20) and for XMM registers (0x60). Save
    // those registers since they also have to be saved on #VMEXIT.
    //
    sub rsp, 0x20 + 0x60
    movaps [rsp + 0x20], xmm0
    movaps [rsp + 0x20 + 0x10], xmm1
    movaps [rsp + 0x20 + 0x20], xmm2
    movaps [rsp + 0x20 + 0x30], xmm3
    movaps [rsp + 0x20 + 0x40], xmm4
    movaps [rsp + 0x20 + 0x50], xmm5

    // Optional: End the function prolog here

    // Handle #VMEXIT
    //
    call handle_vmexit

    // restore XMM registers and roll back stack pointer
    //
    movaps [rsp + 0x20 + 0x50], xmm5
    movaps [rsp + 0x20 + 0x40], xmm4
    movaps [rsp + 0x20 + 0x30], xmm3
    movaps [rsp + 0x20 + 0x20], xmm2
    movaps [rsp + 0x20 + 0x10], xmm1
    movaps [rsp + 0x20], xmm0
    add rsp, 0x20 + 0x60

    // Test the return value of `handle_vmexit` and restore the general purpose
    // registers.
    //
    test al, al

    pop     r15
    pop     r14
    pop     r13
    pop     r12
    pop     r11
    pop     r10
    pop     r9
    pop     r8
    pop     rdi
    pop     rsi
    pop     rbp
    pop     rbx    // Dummy for rsp (this value is destroyed by the next pop).
    pop     rbx
    pop     rdx
    pop     rcx
    pop     rax

    // If the return value is not 0, we need to exit the loop. Otherwise just
    // continue the loop and resume the guest.
    //
    jnz exit_loop       // if (handle_vmexit() != 0 {{ jmp exit_loop }}
    add rsp, 0x190      // else {{ remove trap frame and
    jmp guest_loop      // continue loop }}

exit_loop:
    // Virtualization has been terminated. We have to restore everything back
    // to the original state.
    //
    // Content of some registers (return values of cpuid):
    // - rbx        = address to return
    // - rcx        = original stack pointer (host_rsp)
    //
    mov rsp, rcx        // rsp = host_rsp

    // Update rcx with a magic value, hinting that the hypervisor has been
    // unloaded.
    //
    mov ecx, 0xDEADBEEF

    // Return to the next instruction that triggered the #VMEXIT.
    //
    jmp rbx
