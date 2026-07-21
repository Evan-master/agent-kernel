# Segmented Agent Package V9 Plan

- [x] Audit Capsule parsing, physical identity, page-table mapping, reclamation,
  native fixed addresses, generated image bytes, and QEMU evidence.
- [x] Freeze Package v2 header, segment descriptors, bounded relocation records,
  virtual addresses, and exact frame ordering.
- [x] Add failing parser, relocation, layout, identity, mapping, and pool tests.
- [x] Implement no-heap Package v2 parsing and digest-bound verification.
- [x] Allocate, initialize, relocate, map, reclaim, and reuse rodata frames.
- [x] Rebuild every fixed-address Capsule and migrate Resource Manager to v2.
- [x] Prove segmented execution, R+NX rodata, relocation, and 77-frame recovery.
- [x] Run all host, freestanding, QEMU, ELF, formatting, and audit gates.
- [x] Update bilingual public documentation, commit, and publish `main`.
