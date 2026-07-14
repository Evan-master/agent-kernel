# X86 Exception Baseline V0 Implementation Plan

**Goal:** Install persistent CPU exception gates, prove a returning breakpoint,
and move UART IRQ4 onto the shared architecture-owned IDT.

## Task 1: Gate Contract

- [x] Add a failing host test for the ring-0 trap-gate encoding.
- [x] Implement the allocator-free trap-gate constructor.
- [x] Preserve all existing IDT/PIC encoding tests.

## Task 2: Red QEMU Contract

- [x] Require the exception-baseline success marker.
- [x] Observe the UART-only implementation fail the new marker assertion.

## Task 3: Persistent Exception Runtime

- [x] Add dedicated fatal stubs for exception vectors 0 through 31 except 3.
- [x] Add a returning breakpoint stub that captures the CPU return RIP.
- [x] Install all exception gates into one static 256-entry IDT.
- [x] Validate a real `int3` round trip before Agent Kernel bootstrap.

## Task 4: Shared UART Gate

- [x] Remove IDT storage and `lidt` ownership from the UART adapter.
- [x] Install IRQ4 vector `0x24` into the persistent table with IF clear.
- [x] Preserve the exact 25-event interrupt-to-driver trace.

## Task 5: Documentation And Delivery

- [x] Update README scope, boot handoff, markers, and deferred work.
- [x] Run focused, workspace, no_std, forbidden API, and strict Clippy checks.
- [x] Run QEMU and audit exception/UART disassembly and unsafe boundaries.
- [x] Commit, fast-forward `main`, push, and verify synchronization.
