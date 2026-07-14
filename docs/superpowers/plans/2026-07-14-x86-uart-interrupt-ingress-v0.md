# X86 UART Interrupt Ingress V0 Implementation Plan

**Goal:** Replace the polled COM1 event source with a real IRQ4-to-IDT hardware
interrupt while preserving the kernel Device Event and Driver Invocation model.

## Task 1: Encoding Contract

- [x] Add failing host tests for long-mode IDT entry and IDTR encoding.
- [x] Add failing tests for PIC vector and IRQ4 mask derivation.
- [x] Implement allocator-free architecture interrupt descriptors.

## Task 2: Red QEMU Contract

- [x] Require the UART IRQ success marker.
- [x] Replace the polling-command trace with interrupt-ingress event numbers.
- [x] Observe the old polled implementation fail the new assertions.

## Task 3: Hardware Top Half

- [x] Install a static IDT IRQ4 interrupt gate using the current code selector.
- [x] Remap and mask the 8259 PIC pair with only IRQ4 exposed.
- [x] Arm the 16550 THRE source and enter a bounded interrupt wait.
- [x] Capture IIR/LSR, disable IER, send EOI, and return through `iretq`.

## Task 4: Kernel Bottom Half

- [x] Validate the one-shot IRQ mailbox before creating kernel state.
- [x] Raise an Interrupt Device Event and dispatch/tick its Driver Invocation.
- [x] Execute and terminally record the causal Port write.
- [x] Complete the invocation and verify the Driver returns to idle.

## Task 5: Documentation And Delivery

- [x] Update README behavior, boot handoff, and expected QEMU trace.
- [x] Run focused, workspace, no_std, forbidden API, and strict Clippy checks.
- [x] Run QEMU proof and audit all interrupt-side unsafe boundaries.
- [x] Commit, fast-forward `main`, push, and verify synchronization.
