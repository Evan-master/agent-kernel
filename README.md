# Agent Kernel

Agent Kernel is an early prototype for an agent-native operating system kernel.
It is not a Linux wrapper, shell agent, or POSIX-first compatibility layer.

The project starts from new OS primitives instead of POSIX compatibility:
agents, owned resources, resource lifecycle, capabilities, capability
attenuation, agent launch entries, typed intents, actions, observations,
checkpoints, rollback, verification, tasks, delegation, native mailbox IPC,
task wait signals, task fault traps, fault handlers, fault policies, memory
cells, native object namespace entries, agent execution contexts, and event
logs.

## Current Scope

- `agent-kernel-core`: no_std-friendly agent registry, agent launch entries, agent execution contexts, owned resource creation, resource lifecycle, capability lifecycle, capability attenuation, action, observation, checkpoint, intent store, task store, lifecycle, FIFO run queue, mailbox IPC, task wait signals, task fault traps, fault handlers, fault policies, memory cells, object namespace entries, rollback, and event model.
- `agent-kernel`: no_std kernel facade with syscall-style methods over the core model.
- `agent-kernel-boot`: no_std boot handoff boundary that seeds the kernel with a deterministic bootstrap flow.
- `agent-kernel-x86_64`: no_std x86_64 bootloader entry that emits the boot handoff log over serial.
- `agent-kernel-image`: host-side BIOS image builder and QEMU argument helper.
- `agent-supervisor`: host-side user-space simulator that drives the prototype.

## Current Behavior

The v0 flow is deliberately small:

1. Register the owner, target, and fault handler agents with idle execution contexts.
2. Register a workspace resource.
3. Grant the owner agent a capability for observe, act, verify, checkpoint, rollback, and delegation.
4. Launch the owner agent into a resource-scoped supervisor entry.
5. Install a resource-scoped task fault handler and fault policy.
6. Record the launch, capability grant, and fault automation setup in the kernel event log.
7. Observe the resource and store an observation record.
8. Execute an action with an ActionId and store an action record.
9. Request verification for that action.
10. Create and store a checkpoint record.
11. Request rollback for that checkpoint.
12. Declare a typed action intent that requires verification.
13. Create a kernel-owned task from that intent.
14. Bind the intent to the task.
15. Delegate the task to another agent.
16. Record the derived task-scoped capability in the kernel event log.
17. Let the assignee accept the task.
18. Enqueue the accepted task and dispatch it into `Running` state with a deterministic quantum.
19. Advance the running task by one explicit scheduler tick.
20. Advance it again so the quantum expires and the task is requeued.
21. Redispatch the accepted task from the run queue.
22. Trap the running task into a kernel fault record.
23. Apply the installed fault policy to route the fault to the handler.
24. Let the handler receive and acknowledge the fault message.
25. Recover, requeue, and redispatch the faulted task.
26. Let the assignee wait on a typed workspace signal.
27. Emit the signal and wake the waiting task back into the run queue.
28. Redispatch the woken task.
29. Let the assignee complete the running task.
30. Request verification for the completed task.
31. Mark the intent fulfilled after task verification.
32. Send a native kernel message from the owner agent to the target agent.
33. Let the target agent receive and acknowledge that message.
34. Create a native memory cell under a memory resource.
35. Recall and remember memory cell state through explicit capabilities.
36. Bind a memory cell into the workspace object namespace.
37. Resolve that namespace entry through an observe capability.
38. Rebind the namespace entry to the created task.
39. Create an owned temporary service resource under the workspace.
40. Retire that service resource through its owner capability.
41. Derive an observe-only capability from the owner to the target agent.
42. Let the target agent observe the workspace through that derived capability.
43. Print the kernel event log from the supervisor.

All resource operations go through explicit capabilities. Agent registration,
agent launch, agent suspension, agent resume, agent retirement, owned resource
creation, capability grants, derived root capabilities, derived task capabilities, typed
intent declarations, action, verification, checkpoint creation, rollback
requests, task creation, intent binding, task completion, task verification,
intent fulfillment, delegation, scheduler ticks, quantum expiry, task fault
trapping, fault handler installation, fault policy installation, fault routing,
fault policy application, task fault recovery, task waiting, signal emission,
task wakeup, message send, message receive,
message acknowledgement, memory cell creation, memory recall, memory remember,
namespace bind, namespace resolve, namespace rebind, and resource retirement are
first-class kernel events, not external tooling.
Agents, launch entries, resources, checkpoints, waiters, fault records, fault
handlers, fault policies, messages, memory cells, and namespace entries are also
queryable fixed-capacity kernel records, and new root or derived capabilities
can only be issued to active registered agents. Kernel operations that act on
behalf of an `AgentId` reject unknown, suspended, or retired actors before authorization, state, queue,
mailbox, memory, or capacity checks. Each registered agent has a fixed-capacity
execution context that tracks whether the agent is idle, running a task,
waiting on a signal, or faulted on a task. Launch entries bind an active agent
to a resource, `Act` capability, entry kind, and optional declared action
intent before runtime work proceeds. Dispatch refuses to run a second task for
an agent whose context is already busy. Rollback moves the checkpoint into
`RollbackRequested` status. Accepted tasks move through a fixed-capacity FIFO
run queue, become `Running` with an explicit quantum, accumulate deterministic
ticks, return to the queue when their quantum expires, and can enter `Waiting`
until an authorized signal emission wakes them back into the run queue.
`IntentId`, `TaskId`, and `MessageId` values are allocated by fixed-capacity kernel stores rather than
invented by the supervisor. `WaiterId` and `MemoryCellId` values are also kernel-allocated, and
memory recall writes an audit event before returning a value. Delegation derives
a task-scoped action capability for the assignee, so the supervisor does not
grant broad resource authority to complete delegated work. Revoking the source
capability that authorized delegation also invalidates the derived task-scoped
capability before future task authorization succeeds. Mailbox IPC stores typed
kernel object references instead of heap-allocated bytes or host transport
handles. Memory cells store fixed-width typed words instead of files, byte
buffers, or host persistence. Namespace entries bind compact keys to typed
kernel object references inside workspace resources rather than parsing paths or
delegating lookup to a host filesystem. Retired resources remain queryable for
audit, but active-resource lookup rejects future grants, child resources, and
old-capability operations against them.
General capability derivation lets an agent attenuate its own authority for
another active agent; the derived capability cannot exceed the source
operations, cannot be created from task-scoped authority, and becomes unusable
when the source capability is revoked.
Owner-aware resource creation assigns `owner: Some(agent)` and creates the
first capability atomically with the resource. Bootstrap `register_resource`
remains available for system-seeded resources and leaves `owner: None`.

## Boot Handoff

`agent-kernel-boot` currently validates the kernel-native boot contract:

1. Enter kernel phase.
2. Initialize `AgentKernel`.
3. Register the bootstrap agent.
4. Register a bootstrap resource.
5. Grant observe/act/verify capability to the bootstrap agent.
6. Launch the bootstrap agent into a bootstrap entry.
7. Record observation, action, and verification events.
8. Mark the kernel ready for supervisor handoff.

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
event[3] agent_launched
event[4] observation
event[5] action
event[6] verification
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
event[5] agent_launched agent=1 resource=1 capability=1
event[6] fault_handler_installed agent=1 resource=1 target_agent=3
event[7] fault_policy_installed agent=1 resource=1 policy=1 action=route_to_handler
event[8] observation agent=1 resource=1
event[9] action agent=1 resource=1 action=1
event[10] verification agent=1 resource=1 action=1
event[11] checkpoint agent=1 resource=1 checkpoint=1
event[12] rollback agent=1 resource=1 checkpoint=1
event[13] intent_declared agent=1 resource=1 intent=1
event[14] task_created agent=1 resource=1 task=1
event[15] intent_bound agent=1 resource=1 intent=1
event[16] capability_derived agent=1 resource=1 capability=2
event[17] delegation agent=1 resource=1 task=1 target_agent=2
event[18] task_accepted agent=2 resource=1 task=1
event[19] task_queued agent=2 resource=1 task=1
event[20] task_dispatched agent=2 resource=1 task=1
event[21] task_ticked agent=2 resource=1 task=1 ticks=1 quantum=1
event[22] task_quantum_expired agent=2 resource=1 task=1 ticks=2 quantum=0
event[23] task_dispatched agent=2 resource=1 task=1
event[24] task_faulted agent=2 resource=1 task=1 fault=1 detail=7
event[25] message_sent agent=1 target_agent=3 message=1
event[26] fault_routed agent=1 resource=1 task=1 fault=1 detail=7 target_agent=3 message=1
event[27] fault_policy_applied agent=1 resource=1 task=1 fault=1 detail=7 policy=1 action=route_to_handler message=1
event[28] message_received agent=3 target_agent=1 message=1
event[29] message_acknowledged agent=3 target_agent=1 message=1
event[30] task_fault_recovered agent=1 resource=1 task=1 fault=1 detail=7
event[31] task_queued agent=2 resource=1 task=1
event[32] task_dispatched agent=2 resource=1 task=1
event[33] task_waiting agent=2 resource=1 task=1 waiter=1 signal=1
event[34] signal_emitted agent=1 resource=1 task=1 waiter=1 signal=1 target_agent=2
event[35] task_woken agent=1 resource=1 task=1 waiter=1 signal=1 target_agent=2
event[36] task_dispatched agent=2 resource=1 task=1
event[37] task_completed agent=2 resource=1 task=1
event[38] task_verified agent=1 resource=1 task=1
event[39] intent_fulfilled agent=1 resource=1 intent=1
event[40] message_sent agent=1 target_agent=2 message=2
event[41] message_received agent=2 target_agent=1 message=2
event[42] message_acknowledged agent=2 target_agent=1 message=2
event[43] capability_granted agent=1 resource=2 capability=3
event[44] memory_cell_created agent=1 resource=2 memory_cell=1
event[45] memory_cell_recalled agent=1 resource=2 memory_cell=1
event[46] memory_cell_remembered agent=1 resource=2 memory_cell=1
event[47] namespace_entry_bound agent=1 resource=1 namespace_entry=1 key=1
event[48] namespace_entry_resolved agent=1 resource=1 namespace_entry=1 key=1
event[49] namespace_entry_rebound agent=1 resource=1 namespace_entry=1 key=1
event[50] resource_created agent=1 resource=3 capability=4
event[51] capability_granted agent=1 resource=3 capability=4
event[52] resource_retired agent=1 resource=3 capability=4
event[53] capability_derived agent=1 resource=1 capability=5
event[54] observation agent=2 resource=1
```
