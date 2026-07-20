# Native Namespace Generations V2 Plan

- [x] Audit Namespace revisions, mutation order, fixed-capacity reuse, Agent
  Call headroom, and Resource Manager proof placement.
- [x] Freeze `NamespaceRevisionMismatch`, compare transaction order, Calls 48
  and 49, canonical replies, and Event 188 through 190 evidence.
- [x] Add failing Core and facade contracts for successful compare operations,
  stale generations, authorization, object validation, and Event exhaustion.
- [x] Implement shared Core compare helpers and facade methods without changing
  existing force-operation semantics.
- [x] Add failing x86 contracts, then implement strict decode, authentication,
  canonical replies, CPU acknowledgements, dispatch, and executor paths.
- [x] Replace the Resource Manager force proof with Calls 48 and 49; freeze
  Capsule bytes, digest, offsets, marker counts, and Event 1 through 396.
- [x] Update both README languages and the latest milestone references.
- [x] Run focused tests, workspace tests, Supervisor simulation, `no_std`,
  Clippy, bare-metal checks, debug/release QEMU, and Release ELF audits.
- [x] Publish public `main` while keeping the complete Agent Kernel goal active.
