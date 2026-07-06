# Agent Launch Entry V0 Design

## Purpose

Agent Launch Entry V0 makes agent startup a first-class kernel transition.
The kernel can register agents and track execution contexts, but there is no
native record that says an active agent has been admitted into the runtime
under a specific capability and resource boundary. V0 adds that boundary as an
agent entry record and a replayable launch event.

## Scope

V0 provides:

- `AgentEntryKind::{Bootstrap, Supervisor, Worker}`,
- `AgentEntryRecord { agent, resource, capability, kind, intent }`,
- `launch_agent(agent, capability, resource, kind, intent)`,
- one launch entry per registered agent using the existing `AGENTS` capacity,
- read-only entry inspection through `agent_entries()` and `agent_entry(agent)`,
- facade syscall `sys_launch_agent`,
- `AgentLaunched` event,
- boot handoff, supervisor, QEMU serial, and README coverage.

V0 does not execute model code, load binaries, create host threads, add POSIX
processes, create address spaces, parse prompts, or replace scheduler tasks.
It only records that the kernel accepted an agent into a resource-scoped entry
using explicit authority.

## Core Model

```rust
pub enum AgentEntryKind {
    Bootstrap,
    Supervisor,
    Worker,
}

pub struct AgentEntryRecord {
    pub agent: AgentId,
    pub resource: ResourceId,
    pub capability: CapabilityId,
    pub kind: AgentEntryKind,
    pub intent: Option<IntentId>,
}
```

The entry store is `[AgentEntryRecord; AGENTS]` plus `agent_entry_len`. V0 keeps
capacity tied to registered agents so launch cannot outgrow the agent registry
or introduce another public const generic.

## Launch Contract

`launch_agent` takes:

```rust
launch_agent(
    agent: AgentId,
    capability: CapabilityId,
    resource: ResourceId,
    kind: AgentEntryKind,
    intent: Option<IntentId>,
) -> Result<Event, KernelError>
```

The operation validates:

- the agent is registered and active,
- the agent has not already been launched,
- the capability authorizes `Operation::Act` on the resource for that agent,
- if an intent is supplied, it exists, belongs to the agent, targets the same
  resource, has kind `IntentKind::Act`, and is still `Declared`,
- one launch-entry slot is available,
- one event-log slot is available.

Only after those checks succeed does the core write an `AgentEntryRecord` and
record the launch event.

## Event Model

Successful launch records one event:

```text
AgentLaunched
```

The event stores:

- `agent`,
- `resource`,
- `capability`,
- optional `intent`,
- `target_agent: Some(agent)`.

No existing event is reused because launch is not lifecycle registration,
scheduling, delegation, or task execution. Replay can reconstruct entry records
from `AgentLaunched` events.

## Atomicity And Authority

Failure paths leave the entry store and event log unchanged. `AgentLaunched` is
the event consequence for the only new mutation. Registration still creates the
idle execution context; launch only adds the resource-scoped entry boundary.

## Test Evidence

Tests must prove:

- launching records an entry and `AgentLaunched`,
- optional declared action intents are accepted and stored on the entry/event,
- duplicate launches fail without recording another event,
- inactive or unknown agents cannot launch,
- missing `Act` authority cannot launch,
- intent owner, resource, kind, and status mismatches are rejected without
  partial mutation,
- event-log-full failure leaves no entry,
- the facade exposes launch and entry inspection,
- boot and supervisor flows include the launch event in deterministic output.
