# X86 Native Resource Manager Agent V0 Plan

- [x] Add and observe red tests for canonical operation-set encoding, resource
  Agent Call decode/authentication/replies, and Supervisor Capsule kind 4.
- [x] Implement CreateResource and RetireResource Agent Call ABI operations
  through the existing no-std facade and capability-checked core APIs.
- [x] Build and digest the five-call immutable Resource Manager Capsule.
- [x] Admit Agent 8 with least-authority Workspace delegation and fully private
  memory/CPU ownership.
- [x] Dispatch the Manager through a physical PIT expiry, create and retire one
  Service resource, and validate its exact result and call transcript.
- [x] Require twenty-three dispatches, ten physical expiries, six completions,
  the new marker, and exactly 169 ordered events.
- [x] Update bilingual README architecture, event proof, capacity projection, and
  compatibility/non-goal documentation.
- [x] Run formatting, full tests, Supervisor, no-std checks, scoped Clippy,
  debug/release QEMU, and release disassembly inspection.
- [ ] Commit, merge, publish main, clean the feature branch, and close only this
  milestone while keeping the complete Agent Kernel goal active.
