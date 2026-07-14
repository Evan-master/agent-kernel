# X86 PIT Timer Preemption V0 Implementation Plan

**Goal:** Drive one native Agent Task quantum expiry from a real PIT IRQ0 while
keeping interrupt mechanics outside kernel semantic state.

## Task 1: Timer Contract

- [x] Add failing host tests for PIT ports, mode, divisor, IRQ line, and vector.
- [x] Require timer IRQ and preemption markers in the QEMU proof.
- [x] Require the complete 38-event kernel trace.

## Task 2: Shared Interrupt Hardware

- [x] Extract common 8259 setup and masking from the UART adapter.
- [x] Generalize persistent IDT hardware-gate installation.
- [x] Preserve UART IRQ4 behavior through the shared boundary.

## Task 3: PIT Top Half

- [x] Install vector `0x20` before enabling interrupts.
- [x] Program PIT channel 0 for a bounded 100 Hz Mode 3 tick.
- [x] Capture exactly one IRQ0, mask the PIC, acknowledge it, and return.

## Task 4: Scheduler Bottom Half

- [x] Prepare one admitted Worker Agent and running quantum-one task.
- [x] Apply the validated hardware tick through `sys_tick_task`.
- [x] Verify event, task, run-queue, and execution-context postconditions.

## Task 5: Documentation And Delivery

- [x] Update architecture and boot documentation.
- [x] Run focused, workspace, no_std, forbidden API, and strict Clippy checks.
- [x] Run debug and release QEMU proofs and audit assembly/unsafe boundaries.
- [x] Commit, fast-forward `main`, push, and verify synchronization.
