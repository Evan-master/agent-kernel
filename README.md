# Agent Kernel

Agent Kernel is an early prototype for an agent-native operating system kernel.
It is not a Linux wrapper, shell agent, or POSIX-first compatibility layer.

The project starts from new OS primitives instead of POSIX compatibility:
resources, capabilities, actions, observations, checkpoints, rollback, verification,
and event logs.

## Current Scope

- `agent-kernel-core`: no_std-friendly resource, capability, checkpoint, rollback, and event model.
- `agent-kernel`: no_std kernel facade with syscall-style methods over the core model.
- `agent-supervisor`: host-side user-space simulator that drives the prototype.

## Current Behavior

The v0 flow is deliberately small:

1. Register a workspace resource.
2. Grant an agent a capability for observe, act, verify, checkpoint, and rollback.
3. Observe the resource.
4. Execute an action event with an `ActionId`.
5. Request verification for that action.
6. Create a checkpoint event.
7. Request a rollback event.
8. Print the kernel event log from the supervisor.

All resource operations go through explicit capabilities. Action, verification,
checkpoint, and rollback are first-class kernel events, not external tooling.

## Non-Goals For V0

- Booting on hardware or in QEMU.
- POSIX compatibility.
- Linux syscall compatibility.
- A filesystem, network stack, scheduler, or driver model.
- Running an LLM inside kernel space.

## Commands

```bash
cargo fmt --check
cargo test --workspace
cargo run -p agent-supervisor
```

Expected supervisor output:

```text
Agent Kernel supervisor boot
event[1] observation agent=1 resource=1
event[2] action agent=1 resource=1 action=1
event[3] verification agent=1 resource=1 action=1
event[4] checkpoint agent=1 resource=1 checkpoint=1
event[5] rollback agent=1 resource=1 checkpoint=1
```
