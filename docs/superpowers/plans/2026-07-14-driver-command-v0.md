# Driver Command V0 Implementation Plan

**Goal:** Add a deterministic, capability-authorized output path from a bound
Driver Agent to its device-like resource, including causal device-event links
and terminal command results.

**Architecture:** `agent-kernel-core` owns the fixed-capacity command state
machine and audit events. `agent-kernel` exposes syscall wrappers.
`agent-supervisor` demonstrates the complete event-to-command flow. Kernel
crates remain no_std and perform no hardware or host I/O.

## Task 1: Core Success Path

- [ ] Add a failing end-to-end command lifecycle test.
- [ ] Add `DriverCommandId` and fixed-width command types.
- [ ] Add independent `DRIVER_COMMANDS` capacity and core storage.
- [ ] Add command fields and event kinds to the replay log.
- [ ] Implement submit and terminal transitions.
- [ ] Run the focused lifecycle test.

## Task 2: Authority And Atomicity

- [ ] Test missing binding, wrong driver, denied `Act`, invalid cause, and
  undelivered cause behavior.
- [ ] Test command-store and event-log capacity failures.
- [ ] Test repeated terminal transitions, retired resources, and wrong-driver
  completion.
- [ ] Verify every failure leaves command state and event length unchanged.

## Task 3: Facade And Runtime Trace

- [ ] Add command syscall wrappers and query accessors.
- [ ] Add facade integration coverage.
- [ ] Extend the supervisor flow from acknowledged device event to submitted
  and completed driver command.
- [ ] Format command events in the host supervisor and x86_64 event matcher.
- [ ] Update README primitives, behavior, records, and expected trace.

## Task 4: Verification And Delivery

- [ ] Run formatting and focused core/facade/supervisor tests.
- [ ] Run `cargo test --workspace` and the supervisor binary.
- [ ] Run the QEMU boot verifier.
- [ ] Scan no_std crates for forbidden allocation and host APIs.
- [ ] Inspect the diff, commit scoped changes, merge to `main`, push the private
  remote, and verify a clean synchronized worktree.
