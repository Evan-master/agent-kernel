# X86 Native Memory Concurrency V1 Plan

- [x] Define the concurrent A/B/C sequence, first-fit addresses, observation
  log, capacities, transcript, events, and terminal evidence.
- [x] Add and observe red fixed-capacity observation-log tests.
- [x] Implement the architecture-library observation log and CPU ownership
  transfer.
- [x] Extend the Manager Capsule with concurrent B and C region lifecycles.
- [x] Validate interleaved release, hole reuse, proof ordering, and terminal
  pool state.
- [x] Update capacities, exact events, QEMU markers, and bilingual README.
- [x] Run formatting, full tests, Supervisor, no-std checks, scoped Clippy,
  debug/release QEMU, Capsule extraction, and release disassembly inspection.
- [x] Commit, merge, publish public main, clean the feature branch, and close
  this milestone while keeping the complete Agent Kernel goal active.
