# X86 Agent Task Result Call V0 Plan

- [x] Add failing core contracts for result persistence, replay, authority,
  status, duplicate submission, and event-capacity atomicity.
- [x] Add a failing facade syscall contract.
- [x] Add failing ABI contracts for operation 4, canonical result registers,
  trusted context matching, and success reply encoding.
- [x] Add failing QEMU expectations for result markers and the 57-event flow.
- [x] Implement fixed-width TaskResult state, TaskResultSubmitted events, errors,
  and core/facade submission methods.
- [x] Split the Agent CPU call sequence into bounded result-request,
  acknowledgement, and completion type states.
- [x] Route physical result requests through task-scoped authority without
  changing Running scheduler state.
- [x] Replace both boot Capsules, return offsets, and SHA-256 digests.
- [x] Prove different Worker results, returning mutations, terminal completion,
  and unchanged Driver behavior in QEMU.
- [x] Run formatting, focused/full tests, supervisor, no_std checks, scoped
  Clippy, debug/release QEMU, and release disassembly inspection.
- [x] Update README, commit, and merge locally.
- [ ] Publish local main when GitHub credentials permit.
