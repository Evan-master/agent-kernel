# Agent Registry V0 Design

## Problem

The kernel has typed `AgentId` values, but it does not yet keep a queryable
record that an agent exists. That means the event log can mention agents, but
supervisors and future kernel policies cannot inspect the kernel-owned agent
set directly.

## Decision

Add a fixed-capacity Agent Registry to `agent-kernel-core`.

V0 is intentionally narrow:

- records agent presence as deterministic kernel state,
- emits an `AgentRegistered` event for every successful registration,
- rejects duplicate agent ids,
- rejects capacity exhaustion before mutating state,
- rejects event-log exhaustion before mutating state,
- exposes read-only agent records through the facade,
- makes boot and supervisor flows explicitly register their agents.
- rejects root capability grants to unknown agents,
- rejects task-scoped delegated capability derivation for unknown target agents.

V0 still does not enforce that every existing syscall actor must be a registered
agent. For example, invalid queue operations can still fail on task state rather
than actor registration. The enforced boundary in this step is narrower:
capability authority cannot be issued to an unknown agent, either through a root
grant or through delegated task capability derivation.

## Core Model

```rust
pub enum AgentStatus {
    Active,
}

pub struct AgentRecord {
    pub id: AgentId,
    pub status: AgentStatus,
}
```

`KernelCore` gains an `AGENTS` capacity as the first const generic:

```rust
KernelCore<AGENTS, RESOURCES, CAPS, EVENTS, ACTIONS, OBSERVATIONS, CHECKPOINTS, INTENTS, TASKS, RUN_QUEUE>
```

`register_agent(agent)` performs:

1. duplicate check,
2. agent store capacity check,
3. event capacity check,
4. record write with `AgentStatus::Active`,
5. `AgentRegistered` event append.

Failed registration leaves agent records and events unchanged.

`grant_capability(agent, resource, operations)` now first checks that `agent`
exists in the registry, then checks the resource, capability capacity, and event
capacity. `AgentNotFound` is returned without allocating a capability or
recording an event.

`delegate_task(agent, capability, task, target_agent)` still authorizes the
delegating agent through the source capability, but the internal
`derive_task_capability` step now requires `target_agent` to be registered
before writing the derived capability or mutating the task delegation fields.

## Facade And Runtime

`agent-kernel` adds:

```rust
sys_register_agent(agent) -> Result<Event, KernelError>
agents() -> &[AgentRecord]
```

`agent-kernel-boot` registers the bootstrap agent before granting bootstrap
capabilities.

`agent-supervisor` registers the owner agent and delegated target agent before
creating resources and capabilities.

## Non-Goals

- Agent suspension, retirement, or restart semantics.
- Kernel-allocated agent ids.
- Mandatory registered-actor authorization for all existing syscalls.
- Agent mailboxes, IPC, or scheduling priorities.
- LLM prompts, model sessions, or remote inference in kernel space.

## Test Evidence

- core registration stores `AgentRecord` and records `AgentRegistered`,
- duplicate registration returns `AgentAlreadyExists` without an event,
- store full returns `AgentStoreFull` without an event,
- event log full leaves the registry unchanged,
- root grants to unregistered agents return `AgentNotFound` without an event,
- task delegation to unregistered target agents returns `AgentNotFound` without
  mutating task assignee or delegated capability fields,
- facade exposes registered agents through `agents()`,
- supervisor and QEMU boot still produce deterministic event output.
