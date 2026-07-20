# Native Namespace Manager V1 Plan

- [x] Audit Namespace creation, lookup, authorization, object references,
  monotonic allocation, fixed capacities, and native Capsule headroom.
- [x] Freeze Core retirement semantics, Event tag 87, packed object encoding,
  Agent Calls 44 through 47, and the ring-3 full-capacity proof.
- [x] Add failing Core and boot contracts for retirement atomicity, stable dense
  removal, fresh IDs, authorization, Event evidence, and configurable capacity.
- [x] Implement the Core receipt and transaction, facade, formatter, archive
  tag, and explicit `BootedKernel` Namespace capacity.
- [x] Add failing native ABI contracts, then implement strict decoding,
  authentication, canonical replies, CPU acknowledgements, and dispatch.
- [x] Extend Resource Manager with bind, resolve, rebind, retire, and slot reuse;
  freeze Capsule bytes, digest, offsets, marker counts, and Event 1 through 396.
- [x] Update both README languages and latest milestone references.
- [x] Run focused tests, workspace tests, Supervisor simulation, `no_std`,
  Clippy, bare-metal checks, debug/release QEMU, and release ELF audits.
- [x] Publish public `main` while keeping the complete Agent Kernel goal active.
