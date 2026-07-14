# Driver Endpoint Registry V0 Implementation Plan

**Goal:** Make command dispatch depend on a kernel-owned, validated endpoint
mapping instead of accepting device coordinates from Agent command data.

## Task 1: Model And Store

- [x] Add failing endpoint registration and inspection tests.
- [x] Add endpoint kind, descriptor, and record values.
- [x] Add the fixed `RESOURCES`-capacity store to `KernelCore`.
- [x] Add authorized registration and audit event emission.

## Task 2: Safety Invariants

- [x] Test duplicate resource mappings and unsupported resource kinds.
- [x] Test zero, overflowing, and out-of-range descriptors.
- [x] Test same-kind overlap and separate address spaces.
- [x] Test revoked authority and event-log-full atomicity.

## Task 3: Dispatch And Integration

- [x] Require an endpoint before command dispatch.
- [x] Add facade registration and record inspection.
- [x] Build the supervisor virtual backend from a kernel endpoint record.
- [x] Update trace assertions, x86_64 labels, and README behavior.

## Task 4: Delivery

- [x] Run focused and full workspace tests.
- [x] Run no_std target and forbidden API checks.
- [x] Run strict Clippy and QEMU boot verification.
- [x] Audit, commit, fast-forward `main`, push, and verify synchronization.
