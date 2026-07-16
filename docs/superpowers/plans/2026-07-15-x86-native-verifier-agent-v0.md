# X86 Native Verifier Agent V0 Plan

- [x] Add failing core contracts for Verifier image/entry identity and audited
  TaskResult inspection authorization, state, missing-result, and capacity.
- [x] Add failing facade and native image-loader contracts.
- [x] Add failing ABI contracts for InspectTaskResult and VerifyTask requests
  and replies.
- [x] Add failing QEMU markers and the exact 76-event sequence.
- [x] Implement Verifier kinds, admission, TaskResultInspected, error, and
  core/facade inspection methods.
- [x] Extend Capsule parsing and verification binding to native Verifier images.
- [x] Implement four-call Verifier CPU type states without semantic mutation in
  the architecture layer.
- [x] Implement the scheduled Verifier task adapter and target-only verification.
- [x] Add the immutable Verifier Capsule, offsets, result comparison, and digest.
- [x] Run formatting, focused/full tests, supervisor, no_std checks, scoped
  Clippy, debug/release QEMU, and release disassembly inspection.
- [x] Update README, commit, and merge locally.
- [ ] Publish local main when GitHub credentials permit.
