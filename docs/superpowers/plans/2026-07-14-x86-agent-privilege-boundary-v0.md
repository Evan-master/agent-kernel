# X86 Agent Privilege Boundary V0 Implementation Plan

**Goal:** Execute the admitted Worker at CPL3, preempt it onto a TSS RSP0
stack, resume its exact frame, and accept one bounded Agent yield call.

## Task 1: Red Contracts

- [x] Add failing host tests for GDT, TSS, DPL3 gate, and privilege frame layout.
- [x] Add failing host tests for the fixed user-region layout and proof program.
- [x] Require GDT/TSS, user mapping, CPL3 preemption, and Agent-call markers.

## Task 2: Privilege Foundation

- [x] Add pure descriptor and frame encodings to the architecture library.
- [x] Install and validate a permanent GDT and long-mode TSS.
- [x] Point TSS RSP0 at a fixed privileged entry stack.

## Task 3: User Mapping

- [x] Enable the bootloader physical-memory window.
- [x] Allocate only BootInfo Usable frames through a bounded allocator.
- [x] Map code, signal, guard, and stack pages with least required flags.

## Task 4: Ring-3 Runtime

- [x] Enter CPL3 through a five-word iretq frame.
- [x] Save and validate PIT state on the TSS stack.
- [x] Redispatch, resume, and return through DPL3 interrupt 0x90.

## Task 5: Validation And Delivery

- [x] Preserve events 28 through 30 and the full 40-event trace.
- [x] Update README and architecture documentation.
- [x] Run focused, workspace, no_std, forbidden API, and strict Clippy checks.
- [x] Run debug/release QEMU and audit GDT, entry, IRQ, and call-gate assembly.
- [x] Commit, fast-forward main, push, and verify synchronization.
