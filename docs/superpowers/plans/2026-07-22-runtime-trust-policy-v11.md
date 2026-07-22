# Runtime Trust Policy V11 Plan

- [x] Audit V10 trust verification, core state ownership, Event archive
  encoding, native Agent Call transport, Resource Manager authority, and the
  deferred reuse-worker admission path.
- [x] Freeze signer records, policy generation, atomic rotation, replay fields,
  call `53`, typed-message kind `2`, and QEMU proof markers.
- [x] Add failing core and facade contracts for trust and rotation semantics.
- [x] Add failing typed-message, Agent Call, dynamic loader, and signed reuse
  Worker contracts.
- [x] Implement the fixed-capacity kernel Trust Store and archive format v2.
- [x] Refactor strict Ed25519 verification onto kernel-owned policy records.
- [x] Implement call `53`, Resource Manager rotation, and post-rotation signed
  reuse-worker admission.
- [x] Regenerate signed images from external keys and update exact evidence.
- [x] Pass host, freestanding, Clippy, assembly, ELF, and formatting gates.
- [ ] Pass debug and Release QEMU profiles.
- [ ] Update bilingual public documentation, commit, and publish `main`.
