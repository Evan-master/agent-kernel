# Agent Kernel

Agent Kernel is an early prototype for an agent-native operating system kernel.
It is not a Linux wrapper, shell agent, or POSIX-first compatibility layer.

The project starts from new OS primitives instead of POSIX compatibility:
resources, capabilities, typed intents, actions, observations, checkpoints,
rollback, verification, tasks, delegation, and event logs.

## Current Scope

- `agent-kernel-core`: no_std-friendly resource, capability, action, observation, intent store, task store, lifecycle, FIFO run queue, checkpoint, rollback, and event model.
- `agent-kernel`: no_std kernel facade with syscall-style methods over the core model.
- `agent-kernel-boot`: no_std boot handoff boundary that seeds the kernel with a deterministic bootstrap flow.
- `agent-kernel-x86_64`: no_std x86_64 bootloader entry that emits the boot handoff log over serial.
- `agent-kernel-image`: host-side BIOS image builder and QEMU argument helper.
- `agent-supervisor`: host-side user-space simulator that drives the prototype.

## Current Behavior

The v0 flow is deliberately small:

1. Register a workspace resource.
2. Grant an agent a capability for observe, act, verify, checkpoint, rollback, and delegation.
3. Record the capability grant in the kernel event log.
4. Observe the resource and store an observation record.
5. Execute an action with an ActionId and store an action record.
6. Request verification for that action.
7. Create a checkpoint event.
8. Request a rollback event.
9. Declare a typed action intent that requires verification.
10. Create a kernel-owned task from that intent.
11. Bind the intent to the task.
12. Delegate the task to another agent.
13. Record the derived task-scoped capability in the kernel event log.
14. Let the assignee accept the task.
15. Enqueue the accepted task and dispatch it into `Running` state through the kernel run queue.
16. Let the assignee complete the running task.
17. Request verification for the completed task.
18. Mark the intent fulfilled after task verification.
19. Print the kernel event log from the supervisor.

All resource operations go through explicit capabilities. Capability grants,
derived task capabilities, typed intent declarations, action, verification,
checkpoint, rollback, task creation, intent binding, task completion, task
verification, intent fulfillment, and delegation are first-class kernel events,
not external tooling. Accepted tasks move through a fixed-capacity FIFO run queue
and become `Running` before completion. `IntentId` and `TaskId` values are
allocated by fixed-capacity kernel stores rather than invented by the supervisor.
Delegation derives a task-scoped action capability for the assignee, so the
supervisor does not grant broad resource authority to complete delegated work.
Revoking the source capability that authorized delegation also invalidates the
derived task-scoped capability before future task authorization succeeds.

## Boot Handoff

`agent-kernel-boot` currently validates the kernel-native boot contract:

1. Enter kernel phase.
2. Initialize `AgentKernel`.
3. Register a bootstrap resource.
4. Grant observe/act/verify capability to the bootstrap agent.
5. Record observation, action, and verification events.
6. Mark the kernel ready for supervisor handoff.

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
event[1] capability_granted
event[2] observation
event[3] action
event[4] verification
SUPERVISOR_HANDOFF_READY
```

`scripts/run-qemu.sh` treats QEMU exit code `33` as success because the kernel
exits through the `isa-debug-exit` device with value `0x10`.

Expected supervisor output:

```text
Agent Kernel supervisor boot
event[1] capability_granted agent=1 resource=1 capability=1
event[2] observation agent=1 resource=1
event[3] action agent=1 resource=1 action=1
event[4] verification agent=1 resource=1 action=1
event[5] checkpoint agent=1 resource=1 checkpoint=1
event[6] rollback agent=1 resource=1 checkpoint=1
event[7] intent_declared agent=1 resource=1 intent=1
event[8] task_created agent=1 resource=1 task=1
event[9] intent_bound agent=1 resource=1 intent=1
event[10] capability_derived agent=1 resource=1 capability=2
event[11] delegation agent=1 resource=1 task=1 target_agent=2
event[12] task_accepted agent=2 resource=1 task=1
event[13] task_queued agent=2 resource=1 task=1
event[14] task_dispatched agent=2 resource=1 task=1
event[15] task_completed agent=2 resource=1 task=1
event[16] task_verified agent=1 resource=1 task=1
event[17] intent_fulfilled agent=1 resource=1 intent=1
```
