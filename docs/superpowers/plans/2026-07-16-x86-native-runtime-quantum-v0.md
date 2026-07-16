# X86 Native Runtime Quantum V0 Plan

- [x] Add and observe failing host tests for mutually exclusive native run
  boundary evidence.
- [x] Implement the no_std AgentCall/QuantumExpired boundary classifier.
- [x] Preserve nonce and transcript when a returning call session is preempted.
- [x] Arm and disarm PIT around every prepared, preempted, waiting, and yielded
  ring-3 run.
- [x] Add the read-only per-Agent physical quantum generation counter.
- [x] Extend Worker A to wait for generation 2 after SendMessage and update its
  immutable Capsule digest and return offsets.
- [x] Route post-call expiry through public task tick, physical parking, and the
  same kernel-selected outer dispatch loop.
- [x] Require nine dispatches, four physical expiries, one returning-session
  expiry, the new QEMU marker, and exactly 86 semantic events.
- [x] Update README architecture, boot flow, marker evidence, and non-goals.
- [x] Run formatting, full tests, Supervisor, no_std checks, scoped Clippy,
  debug/release QEMU, and release disassembly inspection.
- [x] Commit, merge, publish main, and close the milestone.
