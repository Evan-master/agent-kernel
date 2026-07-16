# X86 Native Agent Runtime Loop V0 Plan

- [x] Add and observe failing fixed-capacity call transcript tests.
- [x] Implement the host-testable no_std transcript contract.
- [x] Replace role-specific call chains with generic Pending, Resumable,
  Waiting, and Completed CPU ownership states.
- [x] Combine dispatch readiness, semantic commit, and guarded physical take.
- [x] Implement trusted supplemental Verify authority lookup.
- [x] Route all nine ABI V1 operations through one bounded inner call loop.
- [x] Drive Prepared, Preempted, WaitingMailbox, and Yielded contexts through
  one outer kernel-selected dispatch loop.
- [x] Preserve Capsule bytes, digests, return offsets, and the 84-event trace.
- [x] Add the native runtime-loop QEMU marker and update README architecture.
- [x] Run formatting, full tests, Supervisor, no_std checks, scoped Clippy,
  debug/release QEMU, and release disassembly inspection.
- [x] Commit, merge, publish main, and close the milestone.
