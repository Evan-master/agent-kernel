# Action Observation Store V0 Design

## Purpose

The kernel currently records observations, actions, and verification requests as
events, but it does not keep queryable kernel state for the facts those events
describe. This makes the event log readable, but leaves later verifier,
rollback, and resource-lineage primitives without a deterministic object to
inspect.

Action Observation Store V0 adds fixed-capacity records for executed actions and
observations. The goal is not to execute host commands or interpret action
payloads. The goal is to make "what was observed" and "what action was executed"
kernel-visible facts that can be replayed and checked by later primitives.

## Selected Approach

Add dedicated `ActionRecord` and `ObservationRecord` stores to
`agent-kernel-core`, then route observation, action, and verification requests
through those stores.

Alternatives considered:

- Keep deriving action and observation state from events only: this keeps state
  small, but every future verifier or rollback path must replay the log to find
  basic execution facts.
- Add only an action store first: useful for verification, but observation is
  the read-side companion to action and should not remain a second-class event.
- Add verifier records first: verifier records need a stable action target, so
  building verifier state before action state creates an empty shell.

The selected approach keeps the kernel primitive small and deterministic. It
adds record storage and status transitions, but it does not add model calls,
planning, command payloads, host adapters, or rollback policy.

## Architecture Placement

`agent-kernel-core` owns:

- `ActionRecord`,
- `ActionStatus`,
- `ObservationRecord`,
- `ObservationId`,
- fixed-capacity action and observation stores,
- store capacity errors and action lookup errors,
- atomic mutation ordering for observe, act, and verify.

`agent-kernel` owns:

- syscall-style access to `observe`, `act`, and `verify`,
- read-only `actions()` and `observations()` inspection.

`agent-supervisor` owns:

- continuing to drive the existing host-side demonstration flow,
- printing the same event sequence unless output fields are explicitly expanded
  later.

Boot crates own:

- updating generic capacities and exhaustive event construction,
- keeping the QEMU serial output unchanged.

## Data Model

Add `ObservationId` beside the existing typed ids:

```rust
pub struct ObservationId(u64);
```

Add action records:

```rust
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum ActionStatus {
    Executed,
    VerificationRequested,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct ActionRecord {
    pub id: ActionId,
    pub agent: AgentId,
    pub resource: ResourceId,
    pub capability: CapabilityId,
    pub status: ActionStatus,
}
```

Add observation records:

```rust
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct ObservationRecord {
    pub id: ObservationId,
    pub agent: AgentId,
    pub resource: ResourceId,
    pub capability: CapabilityId,
}
```

`ActionId` remains caller-supplied in V0. The current kernel has no deterministic
action payload type, so allocating opaque action ids inside the kernel would only
move the same missing payload problem. V0 instead stores the action id as the
supervisor-provided deterministic action token and rejects duplicate action ids.

`ObservationId` is kernel-allocated because observations currently have no
caller-provided identity.

`KernelCore` gains explicit capacities:

```rust
KernelCore<
    RESOURCES,
    CAPS,
    EVENTS,
    ACTIONS,
    OBSERVATIONS,
    INTENTS,
    TASKS,
    RUN_QUEUE,
>
```

The same capacity order is mirrored by `AgentKernel` and boot helpers.

## Event Model

Extend `Event` with:

```rust
pub observation: Option<ObservationId>,
```

Observation events set:

- `kind: EventKind::Observation`,
- `resource: Some(resource)`,
- `capability: Some(capability)`,
- `operation: Some(Operation::Observe)`,
- `observation: Some(observation_id)`.

Action events continue to set `action: Some(action_id)`.

Verification request events continue to set `action: Some(action_id)`. They also
require that the action record exists and belongs to the same resource.

All existing non-observation events set `observation: None`.

## API Semantics

`observe(agent, capability, resource)`:

- authorizes `Operation::Observe`,
- checks observation store capacity,
- checks for one event slot,
- allocates an `ObservationId`,
- records `ObservationRecord`,
- records `EventKind::Observation`,
- returns the event.

`act(agent, capability, action, resource)`:

- authorizes `Operation::Act`,
- rejects duplicate action ids,
- checks action store capacity,
- checks for one event slot,
- records `ActionRecord` with `ActionStatus::Executed`,
- records `EventKind::ActionExecuted`,
- returns the event.

`verify(agent, capability, action, resource)`:

- authorizes `Operation::Verify`,
- finds the action record,
- requires the action resource to match the supplied resource,
- requires `ActionStatus::Executed`,
- checks for one event slot,
- changes the action status to `VerificationRequested`,
- records `EventKind::VerificationRequested`,
- returns the event.

The public generic `authorize(agent, capability, resource, operation)` event
entry point is removed from the supported core API in V0. It can currently emit
operation events without writing action or observation records, which would
bypass the new stores. Callers must use the specific `observe`, `act`,
`verify`, `checkpoint`, and `rollback` methods instead. The facade already
exposes specific syscall-style methods, so `agent-kernel` keeps the public
`sys_observe` method while switching its implementation to `core.observe(...)`.

## Error Handling

Add:

```rust
ActionStoreFull,
ObservationStoreFull,
ActionAlreadyExists,
ActionNotFound,
ActionResourceMismatch,
ActionStatusMismatch,
```

Use existing errors for authorization and event capacity:

- `OperationDenied`,
- `AgentMismatch`,
- `CapabilityNotFound`,
- `CapabilityRevoked`,
- `ResourceMismatch`,
- `EventLogFull`.

Failed operations must leave records, record counts, id counters, action status,
and event log state unchanged.

## Atomicity Rules

`observe` checks observation capacity and event capacity before incrementing
`next_observation` or writing the record.

`act` checks duplicate id, action capacity, and event capacity before writing the
record.

`verify` checks the action record, resource, status, and event capacity before
changing the action status.

If event recording fails, no store state changes. This mirrors the existing
kernel pattern where each mutating operation must either fully update state and
event log, or do nothing.

## Determinism And Boundaries

Action and observation stores use fixed-capacity arrays, typed ids, explicit
errors, and copyable record structs. They do not use heap allocation, strings,
filesystems, networking, timers, threads, randomness, environment variables, or
host I/O.

The kernel still does not interpret prompts or execute host commands. It records
that an authorized action token was executed against a resource and that an
authorized observation occurred. Planning, natural-language interpretation, and
host integration remain supervisor concerns.

## Tests

Core tests:

- observing a resource stores an `ObservationRecord` and records an observation
  event with `observation: Some(id)`,
- observation store exhaustion returns `ObservationStoreFull` without an event,
- event log exhaustion during observation leaves observation state unchanged,
- acting stores an `ActionRecord` with `ActionStatus::Executed`,
- duplicate action ids return `ActionAlreadyExists` without an event,
- action store exhaustion returns `ActionStoreFull` without an event,
- event log exhaustion during action leaves action state unchanged,
- verifying an existing executed action changes status to
  `VerificationRequested` and records a verification event,
- verifying a missing action returns `ActionNotFound`,
- verifying an action for a different resource returns `ActionResourceMismatch`,
- verifying an already verification-requested action returns
  `ActionStatusMismatch`,
- event log exhaustion during verification leaves action status as `Executed`.

Facade tests:

- `sys_observe` exposes the observation through `observations()`,
- `sys_act` exposes the action through `actions()`,
- `sys_verify` exposes the `VerificationRequested` action status.

Supervisor test:

- existing output remains stable unless a later design expands the printed
  fields.

QEMU test:

- expected serial output remains unchanged.

## Compatibility Impact

`KernelCore` and `AgentKernel` type parameters gain `ACTIONS` and
`OBSERVATIONS`, so test aliases and boot type aliases must be updated.

The core public `authorize(...)` method is replaced by `observe(...)` for the
only supported observation use case. The facade API keeps `sys_observe(...)`, so
supervisor and boot callers continue to use the same facade method.

`Event` gains an `observation` field. Existing event construction sites must set
it to `None` unless they are observation events.

## Non-Goals

V0 does not add:

- action payloads,
- command parsing,
- filesystem or process abstractions,
- verifier result records,
- rollback policy,
- observation contents,
- model calls or prompt handling,
- host adapter execution.

Those require separate designs once the kernel has stable execution facts to
target.

## Self-Review

Placeholder scan: no placeholders remain.

Internal consistency: action ids are supervisor-provided tokens in V0, while
observation ids are kernel-allocated because there is no existing observation
identity. Verification targets existing action records and moves action status
only to `VerificationRequested`, not to a verified outcome.

Scope check: this spec covers one primitive family: deterministic action and
observation records. Verifier results and rollback semantics are intentionally
left for later specs.

Ambiguity check: the generic `authorize(...)` bypass is explicitly removed from
the supported public core API, while the facade-level `sys_observe(...)` remains
stable.
