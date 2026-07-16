# X86 Native Runtime Quantum V0 Plan

- [ ] Add and observe failing host tests for mutually exclusive native run
  boundary evidence.
- [ ] Implement the no_std AgentCall/QuantumExpired boundary classifier.
- [ ] Preserve nonce and transcript when a returning call session is preempted.
- [ ] Arm and disarm PIT around every prepared, preempted, waiting, and yielded
  ring-3 run.
- [ ] Add the read-only per-Agent physical quantum generation counter.
- [ ] Extend Worker A to wait for generation 2 after SendMessage and update its
  immutable Capsule digest and return offsets.
- [ ] Route post-call expiry through public task tick, physical parking, and the
  same kernel-selected outer dispatch loop.
- [ ] Require nine dispatches, four physical expiries, one returning-session
  expiry, the new QEMU marker, and exactly 86 semantic events.
- [ ] Update README architecture, boot flow, marker evidence, and non-goals.
- [ ] Run formatting, full tests, Supervisor, no_std checks, scoped Clippy,
  debug/release QEMU, and release disassembly inspection.
- [ ] Commit, merge, publish main, and close the milestone.
