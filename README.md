# Agent Kernel

Agent Kernel is an early prototype for an agent-native operating system kernel.
It is not a Linux wrapper, shell agent, or POSIX-first compatibility layer.

The project starts from new OS primitives instead of POSIX compatibility:
agents, resources, capabilities, typed intents, actions, observations,
checkpoints, rollback, verification, tasks, delegation, native mailbox IPC,
memory cells, native object namespace entries, and event logs.

## Current Scope

- `agent-kernel-core`: no_std-friendly agent registry, resource, capability, action, observation, checkpoint, intent store, task store, lifecycle, FIFO run queue, mailbox IPC, memory cells, object namespace entries, rollback, and event model.
- `agent-kernel`: no_std kernel facade with syscall-style methods over the core model.
- `agent-kernel-boot`: no_std boot handoff boundary that seeds the kernel with a deterministic bootstrap flow.
- `agent-kernel-x86_64`: no_std x86_64 bootloader entry that emits the boot handoff log over serial.
- `agent-kernel-image`: host-side BIOS image builder and QEMU argument helper.
- `agent-supervisor`: host-side user-space simulator that drives the prototype.

## Current Behavior

The v0 flow is deliberately small:

1. Register the owner and target agents.
2. Register a workspace resource.
3. Grant the owner agent a capability for observe, act, verify, checkpoint, rollback, and delegation.
4. Record the capability grant in the kernel event log.
5. Observe the resource and store an observation record.
6. Execute an action with an ActionId and store an action record.
7. Request verification for that action.
8. Create and store a checkpoint record.
9. Request rollback for that checkpoint.
10. Declare a typed action intent that requires verification.
11. Create a kernel-owned task from that intent.
12. Bind the intent to the task.
13. Delegate the task to another agent.
14. Record the derived task-scoped capability in the kernel event log.
15. Let the assignee accept the task.
16. Enqueue the accepted task and dispatch it into `Running` state with a deterministic quantum.
17. Advance the running task by one explicit scheduler tick.
18. Advance it again so the quantum expires and the task is requeued.
19. Redispatch the accepted task from the run queue.
20. Let the assignee complete the running task.
21. Request verification for the completed task.
22. Mark the intent fulfilled after task verification.
23. Send a native kernel message from the owner agent to the target agent.
24. Let the target agent receive and acknowledge that message.
25. Create a native memory cell under a memory resource.
26. Recall and remember memory cell state through explicit capabilities.
27. Bind a memory cell into the workspace object namespace.
28. Resolve that namespace entry through an observe capability.
29. Rebind the namespace entry to the created task.
30. Print the kernel event log from the supervisor.

All resource operations go through explicit capabilities. Agent registration,
agent suspension, agent resume, agent retirement, capability grants, derived
task capabilities, typed intent declarations, action, verification, checkpoint
creation, rollback requests, task creation, intent binding, task completion,
task verification, intent fulfillment, delegation, scheduler ticks, quantum
expiry, message send, message receive, message acknowledgement, memory cell
creation, memory recall, memory remember, namespace bind, namespace resolve,
and namespace rebind are first-class kernel events, not external tooling.
Agents, checkpoints, messages, memory cells, and namespace entries are also
queryable fixed-capacity kernel
records, and new root or derived capabilities can only be issued to active
registered agents. Kernel operations that act on behalf of an `AgentId` reject
unknown, suspended, or retired actors before authorization, state, queue,
mailbox, memory, or capacity checks. Rollback moves the checkpoint into
`RollbackRequested` status. Accepted tasks move through a fixed-capacity FIFO
run queue, become `Running` with an explicit quantum, accumulate deterministic
ticks, and return to the queue when their quantum expires. `IntentId`, `TaskId`, and
`MessageId` values are allocated by fixed-capacity kernel stores rather than
invented by the supervisor. `MemoryCellId` values are also kernel-allocated, and
memory recall writes an audit event before returning a value. Delegation derives
a task-scoped action capability for the assignee, so the supervisor does not
grant broad resource authority to complete delegated work. Revoking the source
capability that authorized delegation also invalidates the derived task-scoped
capability before future task authorization succeeds. Mailbox IPC stores typed
kernel object references instead of heap-allocated bytes or host transport
handles. Memory cells store fixed-width typed words instead of files, byte
buffers, or host persistence. Namespace entries bind compact keys to typed
kernel object references inside workspace resources rather than parsing paths or
delegating lookup to a host filesystem.

## Boot Handoff

`agent-kernel-boot` currently validates the kernel-native boot contract:

1. Enter kernel phase.
2. Initialize `AgentKernel`.
3. Register the bootstrap agent.
4. Register a bootstrap resource.
5. Grant observe/act/verify capability to the bootstrap agent.
6. Record observation, action, and verification events.
7. Mark the kernel ready for supervisor handoff.

The handoff now runs inside QEMU through the x86_64 BIOS image path.

## Non-Goals For V0

- Booting on physical hardware.
- UEFI image support.
- POSIX compatibility.
- Linux syscall compatibility.
- A filesystem, network stack, preemptive scheduler, or driver model.
- Running an LLM inside kernel space.

## Commands

```bash
PATH="$HOME/.cargo/bin:$PATH" rustup run nightly cargo fmt --check
PATH="$HOME/.cargo/bin:$PATH" RUSTC="$(rustup which rustc --toolchain nightly)" rustup run nightly cargo test --workspace
PATH="$HOME/.cargo/bin:$PATH" RUSTC="$(rustup which rustc --toolchain nightly)" rustup run nightly cargo run -p agent-supervisor
scripts/run-qemu.sh
```

The explicit `PATH` and `RUSTC` prefixes are needed on this local machine
because Homebrew's `cargo` and `rustc` appear earlier in `PATH` than rustup's
toolchain shims.

## QEMU Boot

QEMU is installed through Homebrew:

```bash
brew install qemu
```

Build a BIOS disk image and boot it in QEMU:

```bash
scripts/build-qemu-image.sh
scripts/run-qemu.sh
```

Expected QEMU serial output:

```text
AGENT_KERNEL_QEMU_BOOT_OK
event[1] agent_registered
event[2] capability_granted
event[3] observation
event[4] action
event[5] verification
SUPERVISOR_HANDOFF_READY
```

`scripts/run-qemu.sh` treats QEMU exit code `33` as success because the kernel
exits through the `isa-debug-exit` device with value `0x10`.

Expected supervisor output:

```text
Agent Kernel supervisor boot
event[1] agent_registered agent=1 target_agent=1
event[2] agent_registered agent=2 target_agent=2
event[3] agent_registered agent=3 target_agent=3
event[4] capability_granted agent=1 resource=1 capability=1
event[5] fault_handler_installed agent=1 resource=1 target_agent=3
event[6] observation agent=1 resource=1
event[7] action agent=1 resource=1 action=1
event[8] verification agent=1 resource=1 action=1
event[9] checkpoint agent=1 resource=1 checkpoint=1
event[10] rollback agent=1 resource=1 checkpoint=1
event[11] intent_declared agent=1 resource=1 intent=1
event[12] task_created agent=1 resource=1 task=1
event[13] intent_bound agent=1 resource=1 intent=1
event[14] capability_derived agent=1 resource=1 capability=2
event[15] delegation agent=1 resource=1 task=1 target_agent=2
event[16] task_accepted agent=2 resource=1 task=1
event[17] task_queued agent=2 resource=1 task=1
event[18] task_dispatched agent=2 resource=1 task=1
event[19] task_ticked agent=2 resource=1 task=1 ticks=1 quantum=1
event[20] task_quantum_expired agent=2 resource=1 task=1 ticks=2 quantum=0
event[21] task_dispatched agent=2 resource=1 task=1
event[22] task_faulted agent=2 resource=1 task=1 fault=1 detail=7
event[23] message_sent agent=1 target_agent=3 message=1
event[24] fault_routed agent=1 resource=1 task=1 fault=1 detail=7 target_agent=3 message=1
event[25] message_received agent=3 target_agent=1 message=1
event[26] message_acknowledged agent=3 target_agent=1 message=1
event[27] task_fault_recovered agent=1 resource=1 task=1 fault=1 detail=7
event[28] task_queued agent=2 resource=1 task=1
event[29] task_dispatched agent=2 resource=1 task=1
event[30] task_completed agent=2 resource=1 task=1
event[31] task_verified agent=1 resource=1 task=1
event[32] intent_fulfilled agent=1 resource=1 intent=1
event[33] message_sent agent=1 target_agent=2 message=2
event[34] message_received agent=2 target_agent=1 message=2
event[35] message_acknowledged agent=2 target_agent=1 message=2
event[36] capability_granted agent=1 resource=2 capability=3
event[37] memory_cell_created agent=1 resource=2 memory_cell=1
event[38] memory_cell_recalled agent=1 resource=2 memory_cell=1
event[39] memory_cell_remembered agent=1 resource=2 memory_cell=1
event[40] namespace_entry_bound agent=1 resource=1 namespace_entry=1 key=1
event[41] namespace_entry_resolved agent=1 resource=1 namespace_entry=1 key=1
event[42] namespace_entry_rebound agent=1 resource=1 namespace_entry=1 key=1
```
