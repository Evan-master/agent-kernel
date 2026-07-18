# Runtime Admission Release V1 Plan

- [x] Audit Runtime Admission state, Task verification, physical reclamation,
  event, and facade boundaries.
- [x] Define the generation-bound batch release protocol, physical ordering,
  atomicity contract, capacities, and exact QEMU evidence.
- [x] Add failing core and facade tests for preparation, commit, stale permits,
  readiness, event capacity, and batch atomicity.
- [x] Implement `RuntimeAdmissionReleased`, the opaque batch permit, event
  emission, and facade methods.
- [x] Integrate batch preparation and commit around three-owner physical
  reclamation in the x86 resident flow.
- [x] Update exact evidence, strict QEMU markers, 275-event ordering, and both
  README languages.
- [x] Run the full validation matrix, audit release artifacts, publish public
  `main`, and keep the complete Agent Kernel goal active.
