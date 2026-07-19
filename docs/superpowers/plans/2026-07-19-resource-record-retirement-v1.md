# Resource Record Retirement V1 Plan

- [x] Audit Resource allocation, retirement, dense storage, cleanup authority,
  all non-Event Resource references, x86 capacities, and the Supervisor tail.
- [x] Freeze Capability cleanup revocation, Resource record retirement,
  receipt, Event, Agent Call 41/42, and ring-3 slot-reuse contracts.
- [x] Add failing Core contracts for lifecycle gates, ancestor authorization,
  Capability cleanup, complete references, atomic failures, dense removal,
  stable ordering, and monotonic Resource IDs.
- [x] Implement Core receipts, errors, cleanup revocation, reference preflight,
  dense Resource retirement, and Event archive tag 85.
- [x] Add failing facade and Agent Call 41/42 contracts, then implement strict
  decode, authentication, replies, CPU acknowledgements, and native handlers.
- [x] Extend the Admission Supervisor Capsule to reclaim Resource 3, create
  Resource 8, refill Capability capacity, and preserve the full-log archive.
- [x] Freeze exact Event range, archive digest, Capsule bytes, return offsets,
  marker counts, and merged archived/live transcript.
- [x] Update both README languages and latest milestone references.
- [x] Run focused tests, full workspace tests, Supervisor simulation,
  `no_std` and bare-metal checks, debug/release QEMU, and artifact checks.
- [x] Publish public `main` while keeping the complete Agent Kernel goal active.
