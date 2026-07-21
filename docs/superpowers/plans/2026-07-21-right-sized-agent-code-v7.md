# Right-sized Agent Code Ownership V7 Plan

- [x] Audit Capsule bounds, memory identity, page-table mapping, reclamation,
  runtime admission, rollback, and boot inventory assumptions.
- [x] Freeze the stable four-page virtual window and variable `12..15` physical
  frame profile.
- [x] Add failing identity, reclamation, loader, and layout contracts.
- [x] Implement bounded variable-size identities and exact frame accounting.
- [x] Map only active code pages and reject mappings in the unused code window.
- [x] Seal the initial physical inventory and derive every admission delta from
  its identity.
- [x] Prove heterogeneous one-page and two-page Capsules through native reuse.
- [x] Run focused and Workspace tests, Supervisor simulation, `no_std`, strict
  Clippy, debug and Release QEMU, and Release ELF audits.
- [x] Update public architecture documentation, commit, and publish `main`.
