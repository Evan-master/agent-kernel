# X86 Resumable Runtime Registry V1 Plan

- [ ] Add failing guarded-take ownership and mismatch-atomicity contracts.
- [ ] Add a private prepared/preempted/mailbox-waiting runtime context enum.
- [ ] Preserve rejected non-Copy contexts on every failed park operation.
- [ ] Return every x86 semantic redispatch identity to its physical caller.
- [ ] Park and recover both Worker PIT-preempted contexts through the registry.
- [ ] Park and recover the mailbox receiver across wait and wake.
- [ ] Park and recover the Verifier across PIT preemption.
- [ ] Add the resumable-registry QEMU marker without changing 82 events.
- [ ] Update README architecture scope and future-work boundary.
- [ ] Run formatting, full tests, Supervisor, no_std checks, scoped Clippy,
  debug/release QEMU, and release disassembly inspection.
- [ ] Commit, merge, and publish main.
