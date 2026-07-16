# X86 Dispatch Readiness Handoff V0 Plan

- [ ] Add failing core prepare, commit, and stale-permit contracts.
- [ ] Add failing facade permit delegation contracts.
- [ ] Implement opaque no_std `TaskDispatchPermit` getters and construction.
- [ ] Refactor single-step dispatch through prepare and commit.
- [ ] Add read-only native parked-context readiness checks.
- [ ] Require readiness before all Worker and Verifier dispatch commits.
- [ ] Preserve guarded physical take after every successful commit.
- [ ] Add the dispatch-readiness QEMU marker without changing 82 events.
- [ ] Update README architecture and handoff documentation.
- [ ] Run formatting, full tests, Supervisor, no_std checks, scoped Clippy,
  debug/release QEMU, and release disassembly inspection.
- [ ] Commit, merge, and publish main.
