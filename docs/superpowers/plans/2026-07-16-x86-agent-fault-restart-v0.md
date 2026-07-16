# X86 Agent Fault Restart V0 Plan

- [ ] Add and observe failing host tests for the restart-generation signal ABI.
- [ ] Add restart-generation inspection and mutable signal/stack reset to
  `PreparedAgentMemory` without changing code mappings.
- [ ] Preserve `AgentCpuRuntime` in `FaultedAgentCpu` and add a consuming
  fault-to-prepared restart transition that never resumes the exception frame.
- [ ] Add bounded take semantics for the faulted execution report and register
  the replacement context in the ordinary native runtime.
- [ ] Recover through public `sys_recover_faulted_task` using bootstrap rollback
  authority, then enqueue through public `sys_enqueue_task`.
- [ ] Extend the immutable Fault Worker Capsule to fault at generation 0 and
  issue authenticated describe/complete calls at restart generation 1.
- [ ] Prove fresh admission preemption, one retained immutable fault record,
  `TaskFaultRecovered`, and terminal Fault Worker completion.
- [ ] Require thirteen dispatches, six physical expiries, one fault, four
  completed contexts, no faulted physical contexts, and empty queues.
- [ ] Update the QEMU marker and exact event contract from 101 to 107 events.
- [ ] Update README architecture, boot flow, evidence, and non-goals.
- [ ] Run formatting, focused and full tests, Supervisor, no_std checks, scoped
  Clippy, debug/release QEMU, and release disassembly inspection.
- [ ] Commit, merge, publish main, and close the milestone.
