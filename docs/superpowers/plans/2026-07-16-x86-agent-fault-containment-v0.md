# X86 Agent Fault Containment V0 Plan

- [ ] Add and observe failing host tests for the three-way native run boundary.
- [ ] Add and observe a failing boot test for optional fixed fault capacity.
- [ ] Implement strict `AgentFault(InvalidOpcode)` evidence classification.
- [ ] Expose trailing `FAULTS` capacity through `BootedKernel` without changing
  existing instantiations.
- [ ] Install a CPL-aware vector-6 gate and preserve the original CPL0 fatal
  path.
- [ ] Capture and validate the complete CPL3 #UD frame, CR3, RIP, and mailbox.
- [ ] Add terminal `FaultedAgentCpu` ownership and route it through public
  `sys_fault_task` with `ExecutionTrap` detail 6.
- [ ] Add the immutable Fault Worker Capsule, digest, private address space,
  admission flow, and exact `ud2` offset proof.
- [ ] Queue the Verifier behind the Fault Worker and prove it completes only
  after the fault event.
- [ ] Require eleven dispatches, five physical expiries, one fault boundary,
  the new QEMU marker, and exactly 101 semantic events.
- [ ] Update README architecture, boot flow, evidence, and non-goals.
- [ ] Run formatting, full tests, Supervisor, no_std checks, scoped Clippy,
  debug/release QEMU, and release disassembly inspection.
- [ ] Commit, merge, publish main, and close the milestone.
