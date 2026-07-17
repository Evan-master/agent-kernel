# X86 Native Task Manager V0 Plan

- [x] Record the ABI, authority, capacity, Capsule, and event contracts.
- [x] Add and observe red tests for operations 14 through 16, authentication,
  malformed payload rejection, and canonical replies.
- [x] Implement strict task-lifecycle request decoding and reply encoding.
- [x] Route the three operations through public facade methods in the native
  executor and validate exact state transitions.
- [x] Extend the kind-4 Manager Capsule with declare, create, and delegate
  calls; bind its exact bytes, digest, transcript, and return offsets.
- [x] Raise fixed capacities and require the five ordered lifecycle events.
- [x] Update bilingual README status, ABI tables, and deterministic counts.
- [x] Run formatting, full tests, Supervisor, no-std checks, scoped Clippy,
  debug/release QEMU, Capsule extraction, and disassembly inspection.
- [x] Commit, merge, publish public main, clean the feature branch, and close
  this milestone while keeping the complete Agent Kernel goal active.
