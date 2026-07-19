# Event Archive Checkpoint V1 Plan

- [x] Audit Event sequencing, fixed x86 capacity, authorization helpers,
  Supervisor execution order, and all final evidence dependencies.
- [x] Freeze two-phase proposal/commit semantics, canonical SHA-256 encoding,
  root Rollback authorization, checkpoint chaining, ABI, and full-log proof.
- [x] Add failing Core contracts for deterministic digesting, full-field
  sensitivity, stable prefix removal, sequence continuity, chained commits,
  authorization, stale proposals, and atomic failures.
- [x] Add failing facade and Agent Call 40 contracts.
- [x] Implement canonical Event encoding, proposal construction, checkpoint
  state, Core commit, facade methods, errors, and host formatting.
- [x] Implement Agent Call 40 decode, authentication, canonical reply, CPU
  acknowledgement, native handler, and bounded architecture archive.
- [x] Extend the Admission Supervisor Capsule and prove a 357-slot full Event
  Log archives Events 1 through 64 before execution continues to Event 378.
- [x] Freeze exact digest, Capsule bytes, return offsets, marker counts, and
  merged archived/live Event transcript.
- [x] Update both README languages and latest milestone references.
- [x] Run focused tests, full workspace tests, Supervisor simulation,
  bare-metal check, debug QEMU, release QEMU, and binary occurrence checks.
- [x] Publish public `main` and keep the complete Agent Kernel goal active.
