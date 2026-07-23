# Signed Durable State V13 Plan

- [x] Audit Event Archive commit, digest encoding, HAL, trust policy, and
  Supervisor replay boundaries.
- [x] Freeze durability guarantees, threat model, layer ownership, bounds,
  two-slot transaction, recovery, and rollback-anchor limitations.
- [x] Add failing canonical archive byte-encoding contracts.
- [x] Refactor Event Archive hashing through a sink-based canonical encoder.
- [x] Add fixed-width State Signer, manifest, signature, slot, receipt, and
  recovery value contracts.
- [x] Add Ed25519 manifest verification under a separate State Signer policy.
- [x] Add a fixed-capacity `DurableStateBackend` HAL contract.
- [x] Implement deterministic in-memory dual-slot backend and crash injection.
- [x] Gate Event Archive release on verified flush and readback receipt.
- [x] Reject stale, foreign, duplicate, and replayed receipts atomically.
- [x] Implement dual-slot recovery and anchor-aware head selection.
- [x] Wire the Supervisor reference flow and exact receipt output.
- [x] Pass workspace tests, Supervisor replay, strict Clippy, formatting, and
  no_std x86_64 linking.
- [x] Update bilingual public documentation, commit, and publish the V13 branch.
