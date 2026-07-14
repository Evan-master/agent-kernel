# X86 Port Command Flow V0 Implementation Plan

**Goal:** Produce the physical Port request through the kernel Driver command
state machine and record the backend outcome as a terminal kernel transition.

## Task 1: Boot Capacity Contract

- [x] Add a failing boot test that opts into Driver stores.
- [x] Prove a booted kernel can run a cause-free command to completion.
- [x] Preserve existing ten-argument `BootedKernel` users through defaults.

## Task 2: Physical Command Adapter

- [x] Replace the synthetic Port probe request with kernel setup and dispatch.
- [x] Execute only the immutable request returned by the facade.
- [x] Route completed and failed backend outcomes to matching terminal syscalls.
- [x] Validate the terminal command record before reporting success.

## Task 3: QEMU Trace

- [x] Keep the native Port output marker.
- [x] Add a full command-flow success marker.
- [x] Assert endpoint, Driver, submit, dispatch, and completion event labels.
- [x] Update README behavior and expected output.

## Task 4: Delivery

- [x] Run focused and full workspace tests.
- [x] Run no_std target and forbidden API checks.
- [x] Run strict Clippy and QEMU boot verification.
- [x] Audit, commit, fast-forward `main`, push, and verify synchronization.
