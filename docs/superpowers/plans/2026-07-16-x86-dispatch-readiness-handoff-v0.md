# X86 Dispatch Readiness Handoff V0 Plan

- [x] Add failing core prepare, commit, and stale-permit contracts.
- [x] Add failing facade permit delegation contracts.
- [x] Implement opaque no_std `TaskDispatchPermit` getters and construction.
- [x] Refactor single-step dispatch through prepare and commit.
- [x] Add read-only native parked-context readiness checks.
- [x] Require readiness before all Worker and Verifier dispatch commits.
- [x] Preserve guarded physical take after every successful commit.
- [x] Add the dispatch-readiness QEMU marker without changing 82 events.
- [x] Update README architecture and handoff documentation.
- [x] Run formatting, full tests, Supervisor, no_std checks, scoped Clippy,
  debug/release QEMU, and release disassembly inspection.
- [x] Commit, merge, and publish main.
