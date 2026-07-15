# X86 Agent Task Completion Call V0 Plan

- [x] Add host-failing contracts for CompleteTask decoding, trusted capability
  binding, and wrong-context rejection.
- [x] Add QEMU-failing expectations for completion/authority markers and
  TaskCompleted events 43 and 45.
- [x] Extend Agent Call ABI V1 with terminal CompleteTask.
- [x] Retain and validate each Worker's delegated task capability in trusted
  call and scheduler context.
- [x] Replace Yielded CPU evidence with terminal CompletedAgentCpu evidence.
- [x] Route validated calls through `sys_complete_task` and dispatch only the
  remaining runnable Worker.
- [x] Replace both boot Capsule opcodes and SHA-256 digests.
- [x] Prove both tasks Completed, both execution contexts Idle, and an empty run
  queue without changing Driver terminal semantics.
- [x] Run focused tests, workspace tests, supervisor flow, no_std checks,
  Clippy, formatting, debug/release QEMU, and release disassembly inspection.
- [x] Update README and prepare the verified local commit.
- [ ] Publish the branch when GitHub credentials permit.
