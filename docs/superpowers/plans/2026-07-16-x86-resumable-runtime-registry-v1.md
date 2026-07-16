# X86 Resumable Runtime Registry V1 Plan

- [x] Add failing guarded-take ownership and mismatch-atomicity contracts.
- [x] Add a private prepared/preempted/mailbox-waiting runtime context enum.
- [x] Preserve rejected non-Copy contexts on every failed park operation.
- [x] Return every x86 semantic redispatch identity to its physical caller.
- [x] Park and recover both Worker PIT-preempted contexts through the registry.
- [x] Park and recover the mailbox receiver across wait and wake.
- [x] Park and recover the Verifier across PIT preemption.
- [x] Add the resumable-registry QEMU marker without changing 82 events.
- [x] Update README architecture scope and future-work boundary.
- [x] Run formatting, full tests, Supervisor, no_std checks, scoped Clippy,
  debug/release QEMU, and release disassembly inspection.
- [x] Commit, merge, and publish main.
