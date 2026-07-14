# X86 Driver Invocation Flow V0 Implementation Plan

**Goal:** Turn a kernel-authorized physical COM1 poll into a Device Event,
Driver Invocation, and causally linked physical write command.

## Task 1: Integration Contract

- [x] Add a boot test with device-event, command, and invocation capacities.
- [x] Prove the causal lifecycle reaches acknowledged/completed terminal records.
- [x] Add failing QEMU expectations for the expanded event trace.

## Task 2: Physical Poll

- [x] Dispatch a COM1 line-status `Read` through the kernel command gate.
- [x] Record the read outcome before converting it into a Device Event.
- [x] Reject failed or non-ready physical status without submitting the write.

## Task 3: Driver Invocation

- [x] Deliver the event and dispatch/tick its Driver Invocation.
- [x] Acknowledge the event and submit a causally linked write command.
- [x] Execute only the kernel-returned request and record its terminal outcome.
- [x] Complete and validate the Driver Invocation and execution context.

## Task 4: QEMU Trace

- [x] Add the Driver Invocation success marker.
- [x] Assert polling, event, invocation, causal command, and completion events.
- [x] Update README behavior, boot handoff, and expected serial output.

## Task 5: Delivery

- [x] Run focused and full workspace tests.
- [x] Run no_std target and forbidden API checks.
- [x] Run strict Clippy and QEMU boot verification.
- [x] Audit, commit, fast-forward `main`, push, and verify synchronization.
