# Agent Kernel

Agent Kernel is an early prototype for an agent-native operating system kernel.
It is not a Linux wrapper, shell agent, or POSIX-first compatibility layer.

The project starts from new OS primitives instead of POSIX compatibility:
agents, resources, capabilities, typed intents, actions, observations,
checkpoints, rollback, verification, tasks, delegation, native mailbox IPC, and
event logs.

## Current Scope

- `agent-kernel-core`: no_std-friendly agent registry, resource, capability, action, observation, checkpoint, intent store, task store, lifecycle, FIFO run queue, mailbox IPC, rollback, and event model.
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
16. Enqueue the accepted task and dispatch it into `Running` state through the kernel run queue.
17. Let the assignee complete the running task.
18. Request verification for the completed task.
19. Mark the intent fulfilled after task verification.
20. Send a native kernel message from the owner agent to the target agent.
21. Let the target agent receive and acknowledge that message.
22. Print the kernel event log from the supervisor.

All resource operations go through explicit capabilities. Agent registration,
agent suspension, agent resume, agent retirement, capability grants, derived
task capabilities, typed intent declarations, action, verification, checkpoint
creation, rollback requests, task creation, intent binding, task completion,
task verification, intent fulfillment, delegation, message send, message
receive, and message acknowledgement are first-class kernel events, not external
tooling. Agents, checkpoints, and messages are also queryable fixed-capacity
kernel records, and new root or derived capabilities can only be issued to
active registered agents. Kernel operations that act on behalf of an `AgentId`
reject unknown, suspended, or retired actors before authorization, state, queue,
mailbox, or capacity checks. Rollback moves the checkpoint into
`RollbackRequested` status. Accepted tasks move through a fixed-capacity FIFO
run queue and become `Running` before completion. `IntentId`, `TaskId`, and
`MessageId` values are allocated by fixed-capacity kernel stores rather than
invented by the supervisor. Delegation derives a task-scoped action capability
for the assignee, so the supervisor does not grant broad resource authority to
complete delegated work. Revoking the source capability that authorized
delegation also invalidates the derived task-scoped capability before future
task authorization succeeds. Mailbox IPC stores typed kernel object references
instead of heap-allocated bytes or host transport handles.

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
event[3] capability_granted agent=1 resource=1 capability=1
event[4] observation agent=1 resource=1
event[5] action agent=1 resource=1 action=1
event[6] verification agent=1 resource=1 action=1
event[7] checkpoint agent=1 resource=1 checkpoint=1
event[8] rollback agent=1 resource=1 checkpoint=1
event[9] intent_declared agent=1 resource=1 intent=1
event[10] task_created agent=1 resource=1 task=1
event[11] intent_bound agent=1 resource=1 intent=1
event[12] capability_derived agent=1 resource=1 capability=2
event[13] delegation agent=1 resource=1 task=1 target_agent=2
event[14] task_accepted agent=2 resource=1 task=1
event[15] task_queued agent=2 resource=1 task=1
event[16] task_dispatched agent=2 resource=1 task=1
event[17] task_completed agent=2 resource=1 task=1
event[18] task_verified agent=1 resource=1 task=1
event[19] intent_fulfilled agent=1 resource=1 intent=1
event[20] message_sent agent=1 target_agent=2 message=1
event[21] message_received agent=2 target_agent=1 message=1
event[22] message_acknowledged agent=2 target_agent=1 message=1
```
