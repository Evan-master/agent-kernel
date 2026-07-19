# Waiter Compaction V1 Plan

- [x] Audit Signal and Mailbox waiter creation, wake transitions, references,
  fixed x86 capacity, and retained terminal records.
- [x] Define inactive-prefix eligibility, shared cleanup authority, atomic
  compaction, monotonic identity, receipt, Event, ABI, and x86 slot-reuse proof.
- [x] Add failing Core and facade contracts for Supervisor identity, lifecycle,
  mixed authority, prefix readiness, stable order, Event capacity, monotonic
  IDs, and physical capacity reuse.
- [x] Implement `WaiterCompaction`, Core mutation, facade, errors, complete
  Event evidence, and host formatting.
- [x] Add failing Agent Call 38 contracts and implement decode, authentication,
  execution validation, and canonical reply encoding.
- [x] Extend the Admission Supervisor Capsule with two compaction calls and
  prove Waiter 4 reuse under a three-slot x86 Store.
- [x] Freeze strict 374-Event QEMU sequence, counts, Capsule artifact, and
  release-ELF occurrence.
- [x] Update both README languages and milestone references.
- [x] Run full workspace, Supervisor, bare-metal, debug QEMU, release QEMU, and
  binary-occurrence validation.
- [x] Publish public `main` and keep the complete Agent Kernel goal active.
