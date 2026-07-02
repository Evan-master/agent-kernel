# Delegated Capability Derivation V0 Design

## Purpose

Delegated Capability Derivation V0 makes task delegation carry task-scoped
authority inside the kernel. The current supervisor grants the assignee a
normal resource capability before the assignee can complete delegated work.
That proves the lifecycle, but it keeps a critical authority decision outside
the task delegation primitive.

This design changes delegation so `agent-kernel-core` derives a capability for
the target agent when a task is delegated. The derived capability is scoped to
that task and can authorize completing that task after dispatch. It cannot be
used as a general resource capability.

## Selected Approach

Add task scope to `Capability` and derive a task-scoped `Operation::Act`
capability during `delegate_task`.

Alternatives considered:

- Supervisor grants normal capabilities: simple, but it makes delegated task
  authority a host-side convention instead of a kernel invariant.
- Add a separate `TaskCapability` type: explicit, but it duplicates capability
  storage and authorization logic before the model needs two stores.
- Derive broad resource capabilities for assignees: convenient, but it widens
  authority beyond the delegated task.

Task-scoped capabilities are the right V0 because they reuse the existing
capability store while making least-authority delegation inspectable and
deterministic.

## Architecture Placement

`agent-kernel-core` owns:

- capability task scope,
- derived capability allocation,
- task-scoped authorization checks,
- delegation-time capability derivation,
- storing the derived capability on the task record,
- rejection of task-scoped capabilities for generic resource operations.

`agent-kernel` owns:

- exposing the updated task record through existing read-only `tasks()`,
- no new syscall names in V0.

`agent-supervisor` owns:

- using the derived task capability from the task record,
- no manual grant to the assignee for delegated task completion.

Boot crates stay unchanged in behavior. They compile with the updated
capability shape but do not perform delegation during boot.

## Data Model

Extend `Capability`:

```rust
pub struct Capability {
    pub id: CapabilityId,
    pub agent: AgentId,
    pub resource: ResourceId,
    pub operations: OperationSet,
    pub revoked: bool,
    pub task: Option<TaskId>,
}
```

`task: None` means a normal resource capability. `task: Some(task_id)` means the
capability is scoped to one task and must not authorize generic resource
operations.

Extend `Task`:

```rust
pub struct Task {
    pub id: TaskId,
    pub owner: AgentId,
    pub resource: ResourceId,
    pub assignee: Option<AgentId>,
    pub delegated_capability: Option<CapabilityId>,
    pub status: TaskStatus,
}
```

`delegated_capability` stores the task-scoped capability created for the
assignee. It starts as `None` when a task is created and becomes `Some(id)` when
the task is delegated.

## Operation Semantics

`grant_capability`:

- keeps its current public signature,
- creates normal resource capabilities with `task: None`.

`delegate_task(agent, capability, task, target_agent)`:

- requires the task to exist,
- requires task status `Created`,
- requires the authorizing capability to be normal resource authority for the
  task resource,
- requires the authorizing capability to allow `Operation::Delegate`,
- requires the authorizing capability to allow `Operation::Act` because V0
  derives an action capability,
- checks capability capacity and event capacity before mutating task state,
- creates a derived capability for `target_agent`, the task resource,
  `OperationSet::only(Operation::Act)`, and `task: Some(task)`,
- sets `task.assignee = Some(target_agent)`,
- sets `task.delegated_capability = Some(derived_capability)`,
- sets `task.status = TaskStatus::Delegated`,
- records `DelegationRequested`.

`DelegationRequested` keeps using one event. Its `capability` field stores the
derived task-scoped capability id so the event log exposes the new authority
created by delegation. The source capability is validated by the kernel but is
not stored in the event in V0 because the current event shape has one capability
field.

`complete_task(agent, capability, task)`:

- accepts either a normal resource capability with `task: None` or a
  task-scoped capability whose `task == Some(task)`,
- still requires `Operation::Act`,
- still requires the caller to be the task assignee,
- still requires `TaskStatus::Running`.

Generic resource operations:

- `authorize`, `act`, `verify`, `checkpoint`, and `rollback` reject
  task-scoped capabilities with `CapabilityScopeMismatch`.
- This prevents a delegated task capability from becoming a broad resource
  capability.

`accept_task`, `enqueue_task`, `dispatch_next`, `yield_task`, `verify_task`, and
`cancel_task` keep their existing task lifecycle behavior in V0. Future versions
may derive rollback or verify task capabilities explicitly, but V0 derives only
action authority for delegated completion.

## Error Handling

Add:

```rust
CapabilityScopeMismatch
```

Use `CapabilityScopeMismatch` when:

- a task-scoped capability is used for a generic resource operation,
- a task-scoped capability is used for a different task.

Use existing errors for the rest:

- `CapabilityStoreFull` when delegation cannot allocate the derived capability,
- `OperationDenied` when the source capability cannot delegate or cannot derive
  action authority,
- `TaskStatusMismatch` when the task is not created during delegation,
- `TaskAgentMismatch` when the task lifecycle caller is not the assignee.

Invalid delegation attempts must leave task state, capability state, and event
log unchanged.

## Determinism And Authority

Derivation is deterministic: the derived capability id comes from
`next_capability`, the operations are fixed to `Operation::Act`, and the task
scope is the delegated task id. There is no model call, wall-clock time, host
I/O, randomness, or supervisor mutation.

This design improves least authority. A delegated agent receives authority to
complete the assigned task, not broad authority over the task resource.

## Tests

Implementation must start with failing tests.

Core tests:

- delegating a task stores `delegated_capability` on the task.
- derived capability lets the assignee complete the dispatched task without any
  manual assignee resource grant.
- derived capability cannot be used for generic `act`.
- derived capability cannot complete a different task.
- delegation fails with `OperationDenied` if the source capability lacks
  `Operation::Act` for derived authority.
- delegation returns `CapabilityStoreFull` without task or event mutation when
  there is no capability slot for the derived capability.

Facade tests:

- facade task lifecycle uses the derived task capability from `tasks()`.
- manually granting the assignee resource authority is no longer required in the
  normal lifecycle test.

Supervisor test:

- host flow stops granting a normal assignee capability and still prints the
  same task event sequence.

Full verification:

```bash
PATH="$HOME/.cargo/bin:$PATH" rustup run nightly cargo fmt --check
PATH="$HOME/.cargo/bin:$PATH" RUSTC="$(rustup which rustc --toolchain nightly)" rustup run nightly cargo test --workspace
PATH="$HOME/.cargo/bin:$PATH" RUSTC="$(rustup which rustc --toolchain nightly)" rustup run nightly cargo run -p agent-supervisor
scripts/run-qemu.sh
```

## Compatibility Impact

This design changes the shape of `Capability` and `Task` by adding public
fields. Existing internal initializers and tests must be updated.

This design does not add POSIX, Linux syscall, shell, or legacy compatibility.
It remains an AgentOS-native authority primitive.

## Deferred Work

V0 does not include:

- multiple derived capabilities per task,
- derived rollback or verify authority,
- capability attenuation beyond the fixed `Operation::Act` derivation,
- source capability ids in delegation events,
- replay rebuilding capability state from the event log,
- revocation propagation from source capability to derived capability,
- expiration or leases.

The next step after V0 should be either explicit capability attenuation rules or
source-to-derived revocation propagation.
