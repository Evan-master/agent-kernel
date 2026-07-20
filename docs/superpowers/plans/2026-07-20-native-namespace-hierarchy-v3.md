# Native Namespace Hierarchy V3 Plan

- [x] Audit Namespace object tags, Core capacities, Resource references, Agent
  Call registers, and Resource Manager proof placement.
- [x] Freeze explicit Mount semantics, bounded path order, Call 50, and the
  Events 185 through 190 replacement window.
- [x] Add failing Core contracts for mount validation, cycle prevention,
  per-hop authority, bounded traversal, and Event atomicity.
- [x] Implement the Core Mount object, fixed-store cycle scan, path receipt,
  and one ordered Event per validated hop.
- [x] Add facade contracts and syscall-style path resolution.
- [x] Add failing x86 contracts, then implement Call 50 decode,
  authentication, canonical reply, CPU acknowledgement, and executor path.
- [x] Rebuild the Resource Manager native sequence, capacity profile, semantic
  evidence, Capsule bytes, digest, return offsets, and script markers.
- [x] Run focused tests, Workspace tests, Supervisor simulation, `no_std`,
  strict Clippy, bare-metal checks, debug/release QEMU, and Release ELF audits.
- [x] Update both README languages and publish public `main` while keeping the
  complete Agent Kernel goal active.
