# X86 Agent Image Loader V0 Plan

- [x] Add host-failing contracts for capsule parsing, SHA-256 binding, metadata
  validation, mutation rejection, and distinct Worker payloads.
- [x] Add QEMU-failing expectations for verified load evidence, heterogeneous
  code execution, the shifted scheduler events, and exactly 55 events.
- [x] Implement the fixed 32-byte no_std Agent Image Capsule parser and verified
  image type with explicit loader errors.
- [x] Add two immutable boot capsules and independently precomputed digests.
- [x] Require `VerifiedAgentImage` for code-frame initialization, verify
  physical readback, and enter at the capsule offset.
- [x] Split queued admission from first dispatch so both images load before any
  Worker becomes semantically running.
- [x] Prove different Agent-call return offsets under isolated A/B CR3 roots
  without changing Driver terminal semantics.
- [x] Run focused tests, workspace tests, supervisor flow, no_std checks,
  Clippy, formatting, debug/release QEMU, and release disassembly inspection.
- [x] Update README and prepare the verified local commit.
- [ ] Publish the branch when GitHub credentials permit.
