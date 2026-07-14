# Driver HAL Dispatch V0 Implementation Plan

**Goal:** Require every terminal driver command to pass through an explicit,
replayable HAL dispatch boundary and execute the supervisor flow through a real
backend contract.

## Task 1: State Machine And Request

- [x] Add failing dispatch lifecycle and pre-dispatch terminal tests.
- [x] Add `Dispatched`, `DriverCommandRequest`, and event kind.
- [x] Implement authorized dispatch and immutable request creation.
- [x] Require dispatched state for completion and failure.

## Task 2: Failure And Replay Boundaries

- [x] Test wrong driver, missing command, revoked authority, and retired resource.
- [x] Test repeated dispatch and terminal transitions.
- [x] Test dispatch and terminal event-log-full atomicity.
- [x] Test causal invocation expiry before dispatch.

## Task 3: HAL And Integration

- [x] Add no_std `agent-kernel-hal` workspace crate.
- [x] Add backend outcome contract tests.
- [x] Implement a stateful supervisor virtual device backend.
- [x] Add facade dispatch coverage and migrate command callers.
- [x] Update supervisor trace and x86_64 event labels.

## Task 4: Delivery

- [x] Update README behavior and limitations.
- [x] Run formatting, focused tests, full workspace, no_std, and Clippy checks.
- [x] Run QEMU boot verification.
- [x] Audit, commit, fast-forward `main`, push, and verify synchronization.
