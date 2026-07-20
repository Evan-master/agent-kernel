# Native Namespace Memory Transport V4 Plan

- [x] Audit ring-3 memory layout, CR3 switch order, physical aliases, Agent Call
  decoding, Core depth limits, and native proof ownership.
- [x] Freeze the fixed call-data page, canonical 112-byte record, Call 51
  envelope, single-CPU snapshot rule, and four-hop proof target.
- [x] Add failing user-memory, frame-identity, wire-decoder, and Call 51 ABI
  contracts.
- [x] Implement frame allocation, zeroing, page-table installation, mapping
  validation, reclamation, restart clearing, and bounded snapshot access.
- [x] Implement the pure wire decoder, Call 51 request vocabulary, context
  authentication, canonical reply, CPU acknowledgement, and executor path.
- [x] Extend the Resource Manager with a four-Workspace chain, fixed-page write,
  Call 51 invocation, return validation, and semantic Event evidence.
- [x] Reconcile capacities, event windows, native call counts, return offsets,
  Capsule bytes, hashes, and both README languages.
- [x] Run focused and Workspace tests, Supervisor simulation, `no_std`, strict
  Clippy, bare-metal checks, debug/release QEMU, and Release ELF audits.
- [x] Commit the implementation and publish public `main` while the complete
  Agent Kernel goal remains active.
