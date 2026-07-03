# Intent Lifecycle V0 Design

## Purpose

Intent Store V0 made an agent's objective visible to the kernel, but the intent
record currently stays static after declaration. A task can be created from an
intent and task events carry the intent id, yet the kernel cannot answer whether
the intent is still just declared, bound to work, fulfilled, or cancelled.

Intent Lifecycle V0 adds a deterministic status field to `Intent` and advances
that status through task lifecycle transitions. This makes intent state
replayable and inspectable without moving planning, natural-language
interpretation, or policy into kernel space.

## Selected Approach

Add `IntentStatus` to each intent record and update it automatically from
existing task operations.

Alternatives considered:

- Keep deriving intent state from events only: avoids one field, but every
  caller must replay the log to answer a common kernel-state question.
- Add public intent status syscalls: flexible, but it lets the supervisor bypass
  task lifecycle authority and mutate goals directly.
- Add a full workflow engine: more expressive, but it would mix planning policy
  into the kernel before verifier and resource lineage primitives exist.

The selected approach keeps V0 small: status is a stored kernel fact, task
lifecycle remains the only public mutation path, and all status changes are
recorded as events.

## Architecture Placement

`agent-kernel-core` owns:

- `IntentStatus`,
- status storage on `Intent`,
- status transition helpers,
- status mismatch errors,
- task-driven intent lifecycle events,
- event-capacity checks before task or intent mutation.

`agent-kernel` owns:

- exposing intent status through existing read-only `intents()`,
- keeping task syscalls as the only way to move intent status.

`agent-supervisor` owns:

- printing intent lifecycle events in the host trace.

Boot crates do not declare intents in V0. QEMU boot behavior remains unchanged,
but exhaustive event matches must recognize new intent lifecycle events.

## Data Model

Add:

```rust
pub enum IntentStatus {
    Declared,
    Bound,
    Fulfilled,
    Failed,
    Cancelled,
}
```

Extend `Intent`:

```rust
pub struct Intent {
    pub id: IntentId,
    pub owner: AgentId,
    pub resource: ResourceId,
    pub kind: IntentKind,
    pub verification: VerificationRequirement,
    pub status: IntentStatus,
}
```

`Intent::empty()` uses `IntentStatus::Declared` as the empty-slot status.

`Failed` is reserved for future verifier or rollback failure paths. V0 defines
the type and terminal semantics but does not expose a public failure transition.

## Event Model

Add `EventKind` variants:

```rust
IntentBound,
IntentFulfilled,
IntentCancelled,
```

Each event includes:

- `intent: Some(intent_id)`,
- `task: Some(task_id)`,
- `agent`,
- `resource`,
- `verification` copied from the intent.

Event ordering for the supervisor task flow becomes:

```text
IntentDeclared
TaskCreated
IntentBound
CapabilityDerived
DelegationRequested
TaskAccepted
TaskQueued
TaskDispatched
TaskCompleted
TaskVerified
IntentFulfilled
```

`TaskCreated` and `IntentBound` are separate events. Task creation records the
work entity; intent binding records the goal state transition. This keeps the
event log explicit and leaves room for future multi-task intents.

## Operation Semantics

`declare_intent(...)`:

- creates an intent with `IntentStatus::Declared`,
- records `IntentDeclared`,
- does not bind work.

`create_task(agent, capability, intent)`:

- finds the intent,
- requires `intent.owner == agent`,
- requires `intent.status == IntentStatus::Declared`,
- authorizes `intent.kind.required_operation()` on the intent resource,
- checks task capacity,
- checks for two event slots before mutation,
- creates the task,
- records `TaskCreated`,
- changes intent status to `Bound`,
- records `IntentBound`,
- returns the task id.

If either event cannot be recorded, task state, intent status, and task id
counters remain unchanged.

`verify_task(agent, capability, task)`:

- preserves existing verify authorization and task status checks,
- checks for two event slots before mutation,
- changes the task status to `Verified`,
- records `TaskVerified`,
- changes the task intent status to `Fulfilled`,
- records `IntentFulfilled`.

If either event cannot be recorded, task status and intent status remain
unchanged.

`cancel_task(agent, capability, task)`:

- preserves existing rollback authorization and task status checks,
- checks for two event slots before mutation,
- changes the task status to `Cancelled`,
- records `TaskCancelled`,
- changes the task intent status to `Cancelled`,
- records `IntentCancelled`.

If either event cannot be recorded, task status and intent status remain
unchanged.

Task lifecycle methods that do not complete or cancel work continue to carry the
intent id in their task events but do not change intent status.

## Status Rules

Allowed transitions:

```text
Declared -> Bound
Bound -> Fulfilled
Bound -> Cancelled
```

Terminal statuses:

```text
Fulfilled
Failed
Cancelled
```

Rejected in V0:

- creating a second task from a non-`Declared` intent,
- fulfilling an intent that is not `Bound`,
- cancelling an intent that is not `Bound`,
- changing any terminal intent status.

## Error Handling

Add:

```rust
IntentStatusMismatch
```

Use existing errors for the rest:

- `IntentNotFound`,
- `IntentAgentMismatch`,
- `OperationDenied`,
- `TaskStoreFull`,
- `EventLogFull`,
- task status and authority errors.

Failed transitions must leave the affected intent, task, event log, and id
counters unchanged.

## Determinism And Boundaries

Intent lifecycle uses enum fields, fixed-capacity stores, explicit errors, and
existing event log sequencing. It does not use heap allocation, strings, model
calls, prompt handling, filesystem access, networking, timers, threads,
randomness, or host I/O.

The kernel records lifecycle facts. It does not decide the plan, interpret the
intent, pick a verifier, or retry work. Those remain supervisor concerns until
future kernel primitives make them deterministic.

## Tests

Implementation must start with failing tests.

Core tests:

- declared intents start with `IntentStatus::Declared`,
- creating a task from a declared intent records `TaskCreated` then
  `IntentBound`,
- creating a second task from the same intent returns `IntentStatusMismatch`
  without mutation,
- task verification records `TaskVerified` then `IntentFulfilled`,
- task cancellation records `TaskCancelled` then `IntentCancelled`,
- event-log exhaustion during bind, fulfill, or cancel leaves task and intent
  state unchanged.

Facade tests:

- `sys_create_task` exposes the bound intent status through `intents()`,
- `sys_verify_task` exposes the fulfilled intent status through `intents()`,
- `sys_cancel_task` exposes the cancelled intent status through `intents()`.

Supervisor test:

- host flow prints `intent_bound` after `task_created`,
- host flow prints `intent_fulfilled` after `task_verified`.

Full verification:

```bash
PATH="$HOME/.cargo/bin:$PATH" rustup run nightly cargo fmt --check
PATH="$HOME/.cargo/bin:$PATH" RUSTC="$(rustup which rustc --toolchain nightly)" rustup run nightly cargo test --workspace
PATH="$HOME/.cargo/bin:$PATH" RUSTC="$(rustup which rustc --toolchain nightly)" rustup run nightly cargo run -p agent-supervisor
scripts/run-qemu.sh
```
