# X86 Agent Page Fault V0 Plan

- [x] Add and observe failing host tests for vector-14 classification,
  canonical CR2 addresses, packed semantic detail, and restart generation 3.
- [x] Extend native boundary evidence and fault identity with a bounded page
  error code and exact lower-half user address.
- [x] Add the CR2 mailbox, CPL-aware vector-14 assembly gate, IDT installation,
  error-code frame capture, and kernel-origin fatal fallback.
- [x] Extend mutable-memory reset and fresh-entry preparation to generation 3.
- [x] Assemble and bind a four-path immutable Fault Worker Capsule with exact
  #UD, #GP, #PF, call-return offsets, and SHA-256 digest.
- [x] Prove three ordered semantic records, three authorized recoveries, signal
  page integrity, and final authenticated completion.
- [x] Require seventeen dispatches, eight physical expiries, three faults, four
  completed contexts, no faulted physical contexts, and empty queues.
- [x] Update the QEMU marker and exact event contract from 113 to 119 events.
- [x] Update README architecture, boot flow, evidence, stack contract, and
  non-goals.
- [x] Run formatting, full tests, Supervisor, no_std checks, scoped Clippy,
  debug/release QEMU, and release disassembly inspection.
- [x] Commit, merge, publish main, and close the milestone.
