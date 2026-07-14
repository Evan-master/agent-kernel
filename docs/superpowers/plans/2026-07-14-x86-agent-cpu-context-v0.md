# X86 Agent CPU Context V0 Implementation Plan

**Goal:** Run one admitted Worker on a dedicated x86 stack, preempt it with PIT,
restore its exact interrupt frame after kernel redispatch, and record its yield.

## Task 1: Layout And Red Contracts

- [x] Add failing tests for ABI context and same-ring interrupt frame offsets.
- [x] Require Agent CPU preemption/resume markers in QEMU.
- [x] Expand the deterministic trace from 38 to 40 events.

## Task 2: Cooperative Context Baseline

- [x] Add the fixed aligned Agent stack and context slots.
- [x] Implement ABI-preserving stack switch assembly.
- [x] Enter a native Agent function and prove its stack range/alignment.

## Task 3: Asynchronous PIT Preemption

- [x] Arm PIT from the active Agent stack with IF clear.
- [x] Save all integer registers and same-ring interrupt return state.
- [x] Return to the saved kernel continuation without discarding the Agent frame.

## Task 4: Redispatch And Resume

- [x] Apply quantum expiry and redispatch before CPU resume.
- [x] Restore the interrupt frame through `iretq`.
- [x] Switch cooperatively back and record `TaskYielded`.

## Task 5: Validation And Delivery

- [x] Update README and architecture documentation.
- [x] Run focused, workspace, no_std, forbidden API, and strict Clippy checks.
- [x] Run debug/release QEMU and audit context/IRQ disassembly.
- [x] Commit, fast-forward `main`, push, and verify synchronization.
