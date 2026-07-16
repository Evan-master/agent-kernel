# X86 Agent General Protection V0 Plan

- [ ] Add and observe failing host tests for #GP classification, error-code
  detail encoding, invalid evidence, and the 168-byte privilege frame.
- [ ] Implement host-testable #GP boundary and error-frame contracts while
  preserving the existing #UD detail value.
- [ ] Add the vector-13 mailbox, CPL-aware assembly entry, IDT installation,
  bounded frame read, error validation, and non-resumable normalization.
- [ ] Generalize mutable-memory reset and fault-to-prepared consumption to two
  exact restart generations with generation 2 as the V0 ceiling.
- [ ] Extend the immutable Fault Worker Capsule to execute `ud2`, then `cli`,
  then authenticated describe/complete calls from generations 0, 1, and 2.
- [ ] Split the growing Fault Worker flow by recovery responsibility and prove
  two immutable records, two authorized recoveries, and final completion.
- [ ] Require fifteen dispatches, seven physical expiries, two faults, four
  completed contexts, no faulted physical contexts, and empty queues.
- [ ] Update the QEMU marker and exact event contract from 107 to 113 events.
- [ ] Update README architecture, boot flow, evidence, and non-goals.
- [ ] Run formatting, focused and full tests, Supervisor, no_std checks,
  warnings-denied Clippy, debug/release QEMU, and release disassembly checks.
- [ ] Commit, merge, publish main, and close the milestone.
