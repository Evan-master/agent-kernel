# X86 Native Agent Runtime Loop V0 Plan

- [ ] Add and observe failing fixed-capacity call transcript tests.
- [ ] Implement the host-testable no_std transcript contract.
- [ ] Replace role-specific call chains with generic Pending, Resumable,
  Waiting, and Completed CPU ownership states.
- [ ] Combine dispatch readiness, semantic commit, and guarded physical take.
- [ ] Implement trusted supplemental Verify authority lookup.
- [ ] Route all nine ABI V1 operations through one bounded inner call loop.
- [ ] Drive Prepared, Preempted, WaitingMailbox, and Yielded contexts through
  one outer kernel-selected dispatch loop.
- [ ] Preserve Capsule bytes, digests, return offsets, and the 84-event trace.
- [ ] Add the native runtime-loop QEMU marker and update README architecture.
- [ ] Run formatting, full tests, Supervisor, no_std checks, scoped Clippy,
  debug/release QEMU, and release disassembly inspection.
- [ ] Commit, merge, publish main, and close the milestone.
