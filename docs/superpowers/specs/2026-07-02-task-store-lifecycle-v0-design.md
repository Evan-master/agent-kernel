# Task Store And Lifecycle V0 Design

## Purpose

Task Store And Lifecycle V0 turns `TaskId` from an event-only reference into a
kernel-owned object. The kernel will allocate tasks, track their state, authorize
their transitions, and emit replayable events for every lifecycle mutation.

This keeps Agent Kernel moving toward an agent-native OS model: agents do not
only execute actions; they receive, accept, complete, and verify kernel-visible
work.

## Selected Approach

Use a fixed-capacity task store inside `agent-kernel-core`.

Alternatives considered:

- Event-only task references: already implemented, but the kernel cannot answer
  what state a task is in.
- Supervisor-owned task state: easy to prototype, but it makes task authority a
  user-space convention rather than a kernel invariant.
- Kernel-owned task store: slightly more API surface, but it gives deterministic
  replay, capability checks, and direct lifecycle validation. This is the chosen
  approach.

## Architecture Placement

`agent-kernel-core` owns:

- `Task`
- `TaskStatus`
- fixed-capacity task storage
- task allocation
- lifecycle transition validation
- capability-gated task operations
- event emission for task mutations

`agent-kernel` owns:

- syscall-style wrappers over core task APIs
- no direct task mutation outside the core

`agent-supervisor` owns:

- a host-side demonstration flow
- printing the resulting event log
- no direct mutation of core task internals

Boot crates stay unchanged for this phase. The boot handoff can remain the
smaller observe/action/verify sequence until task scheduling becomes part of
early boot.

## Core Data Model

Add a `Task` model in `agent-kernel-core`:

```rust
pub struct Task {
    pub id: TaskId,
    pub owner: AgentId,
    pub resource: ResourceId,
    pub assignee: Option<AgentId>,
    pub status: TaskStatus,
}
```

Add `TaskStatus`:

```rust
pub enum TaskStatus {
    Created,
    Delegated,
    Accepted,
    Completed,
    Verified,
    Cancelled,
}
```

`TaskId` will be allocated by the kernel from `next_task`. Public APIs should no
longer require callers to invent a task ID when creating a task. Existing
delegation APIs that accept a `TaskId` should verify that the task exists.

`KernelCore` will gain a fourth capacity parameter:

```rust
KernelCore<const RESOURCES: usize, const CAPS: usize, const EVENTS: usize, const TASKS: usize>
```

The same capacity will be mirrored by `AgentKernel`. This is an intentional API
change because tasks are becoming first-class kernel state.

## Operations

Add task-specific core methods:

- `create_task(agent, capability, resource) -> Result<TaskId, KernelError>`
- `delegate_task(agent, capability, task, target_agent) -> Result<Event, KernelError>`
- `accept_task(agent, task) -> Result<Event, KernelError>`
- `complete_task(agent, capability, task) -> Result<Event, KernelError>`
- `verify_task(agent, capability, task) -> Result<Event, KernelError>`
- `cancel_task(agent, capability, task) -> Result<Event, KernelError>`
- `tasks() -> &[Task]`

`create_task` must require access to the resource through a capability. The first
implementation should use `Operation::Act` for task creation rather than adding
a new operation bit. The reason is that V0 does not yet distinguish "describe
work" from "mutate work"; adding a dedicated operation can come once task
intents are modeled.

`delegate_task` must require `Operation::Delegate`.

`complete_task` must require `Operation::Act`.

`verify_task` must require `Operation::Verify`.

`cancel_task` must require `Operation::Rollback` in V0. Cancellation is a
reversal of planned work, and this preserves least-new-surface until rollback
semantics are richer.

`accept_task` is allowed when the caller is the current assignee. It does not
require a capability in V0 because accepting delegated work is not resource
mutation. The task remains bound to its original resource and later work still
requires explicit capabilities.

## Lifecycle Rules

Valid transitions:

- `Created -> Delegated`
- `Created -> Cancelled`
- `Delegated -> Accepted`
- `Delegated -> Cancelled`
- `Accepted -> Completed`
- `Accepted -> Cancelled`
- `Completed -> Verified`
- `Completed -> Cancelled`

Invalid transitions must return explicit errors and must not emit events.

Terminal states:

- `Verified`
- `Cancelled`

Terminal tasks cannot be delegated, accepted, completed, verified again, or
cancelled again.

## Event Model

Extend `EventKind` with:

- `TaskCreated`
- `TaskAccepted`
- `TaskCompleted`
- `TaskVerified`
- `TaskCancelled`

Keep `DelegationRequested` for delegation. Its `task` and `target_agent` fields
will now refer to a real task in the task store.

Every lifecycle mutation must emit exactly one event:

- create emits `TaskCreated`
- delegate emits `DelegationRequested`
- accept emits `TaskAccepted`
- complete emits `TaskCompleted`
- verify emits `TaskVerified`
- cancel emits `TaskCancelled`

Event ordering stays monotonic through the existing event log.

## Error Handling

Add explicit kernel errors:

- `TaskStoreFull`
- `TaskNotFound`
- `TaskAgentMismatch`
- `TaskStatusMismatch`

Capacity failure, lookup failure, assignee mismatch, and invalid lifecycle
transition are normal kernel outcomes. They must return errors rather than
panicking.

Failed task operations must leave both task state and event log unchanged.

## Tests

Implementation must start with failing tests.

Core tests:

- creating a task allocates a kernel-owned `TaskId`, records `TaskCreated`, and
  stores the task as `Created`
- creating a task requires an authorized resource capability
- delegation requires `Operation::Delegate`, updates assignee/status, and records
  `DelegationRequested`
- accepting requires the target agent to match the assignee
- completing requires `Operation::Act` and only works from `Accepted`
- verifying requires `Operation::Verify` and only works from `Completed`
- cancellation requires `Operation::Rollback`
- terminal states reject further transitions without emitting events
- task store capacity returns `TaskStoreFull`

Facade tests:

- syscall wrappers preserve the core task lifecycle behavior
- callers cannot bypass task lifecycle through the facade

Supervisor test:

- host flow prints create, delegate, accept, complete, and verify task events in
  order

Full verification:

```bash
PATH="$HOME/.cargo/bin:$PATH" rustup run nightly cargo fmt --check
PATH="$HOME/.cargo/bin:$PATH" RUSTC="$(rustup which rustc --toolchain nightly)" rustup run nightly cargo test --workspace
PATH="$HOME/.cargo/bin:$PATH" RUSTC="$(rustup which rustc --toolchain nightly)" rustup run nightly cargo run -p agent-supervisor
scripts/run-qemu.sh
```

## Compatibility Impact

This design intentionally changes generic parameters for `KernelCore` and
`AgentKernel` to include task capacity. All internal crates and tests should be
updated in one implementation commit.

There is no POSIX or Linux compatibility impact. This remains an AgentOS-native
primitive.

## Deferred Work

V0 does not include:

- scheduler or run queue
- priority
- deadlines
- task dependency graphs
- task payloads or natural-language intent storage
- child capability derivation during delegation
- persistent replay from event log into state

The immediate next feature after this should be either scheduler/run queue V0 or
delegated capability derivation, depending on whether we want execution order or
authority splitting first.
