# Capability Lifecycle Events V0 Design

## Purpose

Capability Lifecycle Events V0 makes capability creation, derivation, and
revocation visible in the kernel event log. The current kernel records when a
capability is used, but root grants, delegated derivation, and revocation mutate
authority state without event records. That weakens auditability and blocks
future replay from rebuilding capability state.

This design records lifecycle events for root grants, task-scoped derived
capabilities, and revocations while keeping capability validation deterministic
and no_std-compatible.

## Selected Approach

Add explicit capability lifecycle event kinds and extend `Event` with the data
needed to audit capability state changes:

- `CapabilityGranted`
- `CapabilityDerived`
- `CapabilityRevoked`
- `operations: OperationSet`
- `source_capability: Option<CapabilityId>`

Alternatives considered:

- Record only revocation events: closes the immediate invisible mutation gap,
  but the log still cannot explain where a capability came from.
- Reuse `DelegationRequested` as the only derivation event: compact, but it
  mixes task intent with authority creation and cannot represent future
  non-task derivation.
- Add lifecycle events without operation sets or source capability ids:
  easier, but too weak for replay because grant operations and derivation
  parentage are missing.

Explicit lifecycle events with operation sets are the right V0 because they
make authority changes inspectable without introducing policy engines, host I/O,
heap allocation, or legacy OS concepts.

## Architecture Placement

`agent-kernel-core` owns:

- new lifecycle event kinds,
- event fields needed to describe capability operations and parentage,
- event-capacity checks before capability state mutation,
- lifecycle event recording in grant, derivation, and revocation paths,
- atomic failure behavior when lifecycle events cannot be recorded.

`agent-kernel` owns:

- no new syscall names in V0,
- existing facade calls returning the same ids/events,
- tests updated for the expanded event log.

`agent-supervisor` owns:

- formatting lifecycle events in host output,
- larger fixed event capacity for the richer trace.

Boot and architecture crates own:

- printing lifecycle event labels in the boot serial trace,
- tests and README updates for the extra boot event.

## Event Model

Extend `EventKind`:

```rust
pub enum EventKind {
    CapabilityGranted,
    CapabilityDerived,
    CapabilityRevoked,
    Observation,
    ActionExecuted,
    VerificationRequested,
    CheckpointCreated,
    RollbackRequested,
    DelegationRequested,
    TaskCreated,
    TaskAccepted,
    TaskCompleted,
    TaskVerified,
    TaskCancelled,
    TaskQueued,
    TaskDispatched,
    TaskYielded,
}
```

Extend `Event`:

```rust
pub struct Event {
    pub sequence: u64,
    pub agent: AgentId,
    pub kind: EventKind,
    pub resource: Option<ResourceId>,
    pub capability: Option<CapabilityId>,
    pub source_capability: Option<CapabilityId>,
    pub action: Option<ActionId>,
    pub operation: Option<Operation>,
    pub operations: OperationSet,
    pub checkpoint: Option<CheckpointId>,
    pub task: Option<TaskId>,
    pub target_agent: Option<AgentId>,
}
```

`operation` keeps describing the single operation being requested or executed.
`operations` describes the operation set carried by a capability lifecycle
event. Non-lifecycle events set `operations` to `OperationSet::empty()`.

`source_capability` identifies the parent capability used to derive a new
capability. Root grants and revocations set it to `None`.

## Operation Semantics

`grant_capability(agent, resource, operations)`:

- validates the resource,
- checks for one free capability slot,
- checks for one free event slot,
- allocates the root capability,
- records `CapabilityGranted`,
- returns the new capability id.

The grant event uses:

- `agent`: capability holder,
- `resource`: granted resource,
- `capability`: new capability id,
- `operations`: granted operation set,
- `source_capability`: `None`,
- `task`: `None`,
- `target_agent`: `None`.

`derive_task_capability(target_agent, resource, operations, task, parent)`:

- validates the resource,
- validates the parent capability id,
- checks for one free capability slot,
- checks for one free event slot,
- allocates the task-scoped capability,
- records `CapabilityDerived`,
- returns the derived capability id.

The derived event uses:

- `agent`: source capability holder,
- `target_agent`: derived capability holder,
- `resource`: delegated resource,
- `capability`: derived capability id,
- `source_capability`: parent capability id,
- `operations`: derived operation set,
- `task`: task scope.

The source capability holder is read from the parent capability record. The
`target_agent` argument remains the holder of the newly derived capability.

`delegate_task(agent, capability, task, target_agent)`:

- keeps the current authorization and task status checks,
- checks that two event slots are available before mutation because derivation
  and delegation each record one event,
- derives the task capability,
- mutates task assignee, delegated capability, and status,
- records `DelegationRequested`.

If there is only one free event slot when delegation needs derivation, the whole
delegation fails with `EventLogFull` and leaves capability state, task state, and
event log unchanged.

`revoke_capability(capability)`:

- finds the capability,
- checks for one free event slot,
- marks the capability revoked,
- records `CapabilityRevoked`.

The revoke event uses:

- `agent`: revoked capability holder,
- `resource`: revoked capability resource,
- `capability`: revoked capability id,
- `operations`: revoked capability operation set,
- `task`: revoked task scope, if any,
- `source_capability`: `None`,
- `target_agent`: `None`.

V0 keeps the public revoke signature unchanged. A future authorization model
should add an explicit revocation actor and revocation capability class.

## Atomicity And Errors

Capability lifecycle operations must check event capacity before mutating
capability or task state.

Use existing errors:

- `ResourceNotFound` for missing resources,
- `CapabilityNotFound` for missing capability or parent ids,
- `CapabilityStoreFull` for no free capability slot,
- `EventLogFull` when the lifecycle event cannot be recorded.

Failed lifecycle operations must leave:

- capability store unchanged,
- task state unchanged,
- event log unchanged,
- id counters unchanged when no capability was allocated.

## Determinism And Boundaries

Lifecycle event recording uses only fixed-capacity kernel stores and typed ids.
There is no allocation, filesystem access, host I/O, model call, randomness, or
wall-clock time in `agent-kernel-core` or `agent-kernel`.

This remains an AgentOS-native authority model. It does not add users, POSIX
permissions, Linux syscalls, filesystems, sockets, or process abstractions.

## Tests

Implementation must start with failing tests.

Core tests:

- root grants record `CapabilityGranted` with operation set and capability id,
- grant returns `EventLogFull` without allocating when no event slot exists,
- revocation records `CapabilityRevoked`,
- revoke returns `EventLogFull` without revoking when no event slot exists,
- delegation records `CapabilityDerived` before `DelegationRequested`,
- delegation with only one free event slot fails without deriving capability or
  mutating task state.

Existing tests must be updated so failure assertions compare against the event
count immediately before the failed operation rather than assuming grants are
invisible.

Facade tests:

- `sys_grant` records the grant event,
- task lifecycle tests expect grant and derived events in sequence.

Supervisor and boot tests:

- supervisor output includes capability lifecycle events,
- QEMU serial output includes `capability_granted` before observation.

Full verification:

```bash
PATH="$HOME/.cargo/bin:$PATH" rustup run nightly cargo fmt --check
PATH="$HOME/.cargo/bin:$PATH" RUSTC="$(rustup which rustc --toolchain nightly)" rustup run nightly cargo test --workspace
PATH="$HOME/.cargo/bin:$PATH" RUSTC="$(rustup which rustc --toolchain nightly)" rustup run nightly cargo run -p agent-supervisor
scripts/run-qemu.sh
```

## Compatibility Impact

This design changes event ordering and event counts because capability grants
and derived grants become visible events. Existing tests, README traces,
supervisor output, and QEMU serial expectations must be updated.

The public `Event` struct gains `source_capability` and `operations` fields.
The public `grant_capability`, `revoke_capability`, and facade method signatures
remain unchanged.

## Deferred Work

V0 does not include:

- authorized revoke actors,
- grant issuers distinct from capability holders,
- replay engine implementation,
- lifecycle events for resource registration,
- capability expiration or leases,
- querying lifecycle history by capability id.
