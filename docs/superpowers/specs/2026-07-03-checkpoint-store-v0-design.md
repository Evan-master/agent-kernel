# Checkpoint Store V0 Design

## Purpose

The kernel currently records checkpoint and rollback requests as events, but it
does not keep queryable checkpoint state. That keeps the event log readable, but
it leaves rollback, verifier, and future resource-lineage primitives without a
deterministic object to inspect.

Checkpoint Store V0 makes checkpoints first-class kernel records. The goal is
not to restore memory, files, devices, or host state. The goal is to make "a
checkpoint exists for this resource" and "rollback has been requested for that
checkpoint" visible as fixed-capacity kernel state.

## Selected Approach

Add a dedicated `CheckpointRecord` store to `agent-kernel-core`, then route
checkpoint creation and rollback requests through that store.

Alternatives considered:

- Keep deriving checkpoint state from events only: this avoids a new capacity,
  but every rollback or verifier path must replay the full event log to know
  whether a checkpoint exists.
- Treat rollback as a completed restore: the current kernel has no resource
  snapshot payload or driver model, so marking rollback as applied would
  overstate the primitive.
- Allocate checkpoint ids inside the kernel: useful later, but the current
  public API already accepts deterministic `CheckpointId` tokens and the boot
  and supervisor flows use them. V0 keeps caller-provided ids and rejects
  duplicates.

The selected approach keeps the primitive small and replayable. It adds record
storage and status transitions, but it does not add resource snapshots, storage
drivers, payload diffing, filesystem semantics, or host rollback policy.

## Architecture Placement

`agent-kernel-core` owns:

- `CheckpointRecord`,
- `CheckpointStatus`,
- a fixed-capacity checkpoint store,
- checkpoint lookup and status validation errors,
- atomic mutation ordering for checkpoint and rollback.

`agent-kernel` owns:

- syscall-style access to `checkpoint` and `rollback`,
- read-only `checkpoints()` inspection.

`agent-supervisor` owns:

- continuing to drive the host-side demonstration flow,
- printing the same checkpoint and rollback event lines unless output fields
  are explicitly expanded later.

Boot crates own:

- updating generic capacities and boot aliases,
- keeping the QEMU serial output unchanged.

## Data Model

Add checkpoint records:

```rust
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum CheckpointStatus {
    Created,
    RollbackRequested,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct CheckpointRecord {
    pub id: CheckpointId,
    pub agent: AgentId,
    pub resource: ResourceId,
    pub capability: CapabilityId,
    pub status: CheckpointStatus,
}
```

`CheckpointId` remains caller-supplied in V0. The kernel rejects duplicate
checkpoint ids so later rollback requests target a single record.

`KernelCore` gains an explicit checkpoint capacity between `OBSERVATIONS` and
`INTENTS`:

```rust
KernelCore<
    RESOURCES,
    CAPS,
    EVENTS,
    ACTIONS,
    OBSERVATIONS,
    CHECKPOINTS,
    INTENTS,
    TASKS,
    RUN_QUEUE,
>
```

The same capacity order is mirrored by `AgentKernel` and boot helpers.

## API Semantics

`checkpoint(agent, capability, checkpoint, resource)`:

- authorizes `Operation::Checkpoint`,
- rejects duplicate checkpoint ids,
- checks checkpoint store capacity,
- checks for one event slot,
- records `CheckpointRecord` with `CheckpointStatus::Created`,
- records `EventKind::CheckpointCreated`,
- returns the event.

`rollback(agent, capability, checkpoint, resource)`:

- authorizes `Operation::Rollback`,
- finds the checkpoint record,
- requires the checkpoint resource to match the supplied resource,
- requires `CheckpointStatus::Created`,
- checks for one event slot,
- changes the checkpoint status to `RollbackRequested`,
- records `EventKind::RollbackRequested`,
- returns the event.

`rollback` is a request lifecycle transition in V0. It does not claim the
resource has been restored.

## Error Handling

Add:

```rust
CheckpointStoreFull,
CheckpointAlreadyExists,
CheckpointNotFound,
CheckpointResourceMismatch,
CheckpointStatusMismatch,
```

Use existing errors for authorization and event capacity:

- `OperationDenied`,
- `AgentMismatch`,
- `CapabilityNotFound`,
- `CapabilityRevoked`,
- `ResourceMismatch`,
- `EventLogFull`.

Failed operations must leave records, record counts, checkpoint status, and
event log state unchanged.

## Atomicity Rules

`checkpoint` checks duplicate id, checkpoint capacity, and event capacity before
writing the record.

`rollback` checks the checkpoint record, resource, status, and event capacity
before changing checkpoint status.

If event recording fails, no checkpoint store state changes. This mirrors the
existing kernel pattern where each mutating operation must either fully update
state and event log, or do nothing.

## Determinism And Boundaries

Checkpoint records use fixed-capacity arrays, typed ids, explicit errors, and
copyable record structs. They do not use heap allocation, strings, filesystems,
networking, timers, threads, randomness, environment variables, or host I/O.

The kernel still does not snapshot or restore host resources. Snapshot payloads,
resource-specific rollback policy, filesystem adapters, and device rollback
belong to later designs.

## Tests

Core tests:

- creating a checkpoint stores a `CheckpointRecord` and records a checkpoint
  event,
- duplicate checkpoint ids return `CheckpointAlreadyExists` without an event,
- checkpoint store exhaustion returns `CheckpointStoreFull` without an event,
- event log exhaustion during checkpoint creation leaves checkpoint state
  unchanged,
- rolling back an existing created checkpoint changes status to
  `RollbackRequested` and records a rollback event,
- rolling back a missing checkpoint returns `CheckpointNotFound`,
- rolling back a checkpoint for a different resource returns
  `CheckpointResourceMismatch`,
- rolling back an already rollback-requested checkpoint returns
  `CheckpointStatusMismatch`,
- event log exhaustion during rollback leaves checkpoint status as `Created`.

Facade tests:

- `sys_checkpoint` exposes the checkpoint through `checkpoints()`,
- `sys_rollback` exposes the `RollbackRequested` checkpoint status.

Supervisor and QEMU tests:

- existing output remains stable.

## Compatibility Impact

`KernelCore`, `AgentKernel`, and `BootedKernel` type parameters gain
`CHECKPOINTS`, so test aliases and boot type aliases must be updated.

The existing `checkpoint(...)`, `rollback(...)`, `sys_checkpoint(...)`, and
`sys_rollback(...)` method signatures remain stable.

## Non-Goals

V0 does not add:

- resource snapshot payloads,
- filesystem or process abstractions,
- rollback execution policy,
- checkpoint id allocation,
- checkpoint garbage collection,
- rollback completion or failure outcomes,
- host adapter execution.

Those require separate designs once the kernel has stable checkpoint facts to
target.

## Self-Review

Placeholder scan: no placeholders remain.

Internal consistency: checkpoint ids are caller-provided tokens in V0, matching
the existing facade and supervisor flows. Rollback targets existing checkpoint
records and moves status only to `RollbackRequested`, not to an applied restore.

Scope check: this spec covers one primitive family: deterministic checkpoint
records and rollback-request lifecycle. Snapshot payloads and rollback policy
are intentionally left for later specs.
