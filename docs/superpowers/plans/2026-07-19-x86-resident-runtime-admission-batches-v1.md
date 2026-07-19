# X86 Resident Runtime Admission Batches V1 Plan

- [x] Audit the resident Supervisor, Runtime Admission store, frame pool,
  partial reclamation boundary, capacities, and exact evidence.
- [x] Define two FIFO batches, fixed identities, two Mailbox waits, partial
  release, cross-batch frame reuse, terminal release, and failure behavior.
- [x] Add failing frame-ledger and strict QEMU multi-batch contracts.
- [x] Generalize completed CPU address-space reclamation for a verified partial
  batch while preserving atomic ownership transfer.
- [x] Extend the ring-3 Supervisor Capsule and x86 flow through two sequential
  admission, notification, verification, reclamation, and release rounds.
- [x] Freeze Capsule authority and update exact counters, marker contracts, and
  both README languages.
- [ ] Run the complete validation and artifact audit, publish public `main`, and
  keep the complete Agent Kernel goal active.
