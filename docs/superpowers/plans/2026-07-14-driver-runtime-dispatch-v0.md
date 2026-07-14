# Driver Runtime Dispatch V0 Implementation Plan

**Goal:** Make delivered device events create deterministic runnable work for a
verified, launched Driver Agent and carry that work through dispatch, quantum,
acknowledgement, command response, and completion.

**Architecture:** `agent-kernel-core` owns Driver Invocation records, admission,
FIFO selection, execution-context transitions, and events. `agent-kernel`
exposes syscall methods. `agent-supervisor` drives explicit ticks and simulated
driver behavior without mutating core state.

## Task 1: Runtime Identity And Success Path

- [x] Add a failing end-to-end invocation lifecycle test.
- [x] Add Driver image and entry kinds.
- [x] Add invocation ID, record, status, capacity, and event fields.
- [x] Extend execution contexts with mutually exclusive driver work.
- [x] Implement atomic delivery/queueing, dispatch, acknowledgement, command
  causality, and completion.

## Task 2: Scheduler And Atomicity

- [x] Test delivery capacity and two-event atomicity.
- [x] Test missing/wrong launch entry and revoked launch authority.
- [x] Test per-driver FIFO and busy execution contexts.
- [x] Test tick progress, quantum expiry, and deterministic redispatch.
- [x] Test acknowledgement and command rejection before dispatch.
- [x] Test completion before acknowledgement and event-log-full behavior.
- [x] Test retired resources and wrong drivers without mutation.

## Task 3: Facade And Supervisor

- [x] Add runtime dispatch syscalls and invocation inspection.
- [x] Add facade integration coverage.
- [x] Launch a fourth supervisor-simulated Driver Agent from a verified Driver
  image and drive the full invocation flow.
- [x] Format invocation and Driver image events in supervisor and x86_64 paths.
- [x] Update README primitives, behavior, records, limitations, and trace.

## Task 4: Verification And Delivery

- [x] Run formatting, focused tests, full workspace tests, and supervisor.
- [x] Run QEMU boot verification.
- [x] Scan all no_std crates for forbidden allocation and host APIs.
- [x] Audit the diff, commit, fast-forward `main`, push the private remote, and
  verify local/remote synchronization.
