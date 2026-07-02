# Intent Store V0 Design

## Purpose

Intent Store V0 makes "what the agent is trying to achieve" a kernel-visible
primitive instead of leaving tasks as resource-only lifecycle records. The
current task model can say who owns work, which resource it touches, who it was
delegated to, and whether it ran. It cannot say whether the work was an
observation, action, checkpoint, rollback, or verification intent, and it cannot
record whether that intent requires verification.

This design adds a fixed-capacity no_std intent store. Agents declare typed
intents through explicit capabilities, and tasks are created from declared
intents.

## Selected Approach

Add first-class `Intent` records with typed IDs and bind every new task to one
intent.

Alternatives considered:

- Store intent fields directly on `Task`: compact, but it makes intent exist
  only after scheduling work and prevents future intent reuse, inspection, or
  replay as an independent primitive.
- Store natural-language intent text: closer to human prompts, but it requires
  allocation, parsing policy, and model interpretation inside or near the
  kernel. That violates the kernel boundary.
- Keep tasks resource-only and let the supervisor remember intent: simple, but
  it preserves the current gap where the kernel cannot inspect the actual
  objective.

An explicit intent store is the right V0 because it is deterministic, replayable,
and still small enough to audit.

## Architecture Placement

`agent-kernel-core` owns:

- `IntentId`, `Intent`, `IntentKind`, and `VerificationRequirement`,
- fixed-capacity intent storage and lookup,
- capability-gated intent declaration,
- binding task creation to an existing intent,
- intent metadata in task and event records,
- event-capacity checks before intent or task mutation.

`agent-kernel` owns:

- syscall-style facade methods for declaring intents and creating tasks from
  intents,
- read-only intent inspection through `intents()`.

`agent-supervisor` owns:

- declaring an action intent before creating a task,
- printing intent-related events in the host trace.

Boot crates stay unchanged in behavior in V0. The boot handoff remains a small
resource/capability/action/verification sequence and does not create tasks or
intents.

## Data Model

Add `IntentId`:

```rust
pub struct IntentId(u64);
```

Add `IntentKind`:

```rust
pub enum IntentKind {
    Observe,
    Act,
    Verify,
    Checkpoint,
    Rollback,
}
```

Each kind maps to the operation required to declare it:

- `Observe` -> `Operation::Observe`
- `Act` -> `Operation::Act`
- `Verify` -> `Operation::Verify`
- `Checkpoint` -> `Operation::Checkpoint`
- `Rollback` -> `Operation::Rollback`

`Operation::Delegate` is intentionally excluded. Delegation is authority
routing for work, not an objective against a resource.

Add `VerificationRequirement`:

```rust
pub enum VerificationRequirement {
    Optional,
    Required,
}
```

Add `Intent`:

```rust
pub struct Intent {
    pub id: IntentId,
    pub owner: AgentId,
    pub resource: ResourceId,
    pub kind: IntentKind,
    pub verification: VerificationRequirement,
}
```

Extend `Task`:

```rust
pub struct Task {
    pub id: TaskId,
    pub intent: IntentId,
    pub owner: AgentId,
    pub resource: ResourceId,
    pub assignee: Option<AgentId>,
    pub delegated_capability: Option<CapabilityId>,
    pub status: TaskStatus,
}
```

Extend `Event`:

```rust
pub struct Event {
    pub intent: Option<IntentId>,
    pub intent_kind: Option<IntentKind>,
    pub verification: VerificationRequirement,
    // existing fields remain
}
```

Non-intent events use `intent: None`, `intent_kind: None`, and
`VerificationRequirement::Optional`.

## Operation Semantics

`declare_intent(agent, capability, resource, kind, verification)`:

- validates the resource,
- requires the capability to authorize `kind.required_operation()` on the
  resource,
- checks intent store capacity,
- checks event capacity,
- allocates an `IntentId`,
- stores the intent,
- records `IntentDeclared`,
- returns the intent id.

`IntentDeclared` includes:

- owner agent,
- resource,
- authorizing capability,
- intent id,
- intent kind,
- required operation,
- verification requirement.

`create_task(agent, capability, intent)`:

- replaces the current resource-based public core method,
- finds the intent,
- requires `agent == intent.owner`,
- requires the capability to authorize `intent.kind.required_operation()` on
  the intent resource,
- checks task store capacity,
- checks event capacity,
- creates the task with `Task.intent = intent`,
- copies `Task.resource` from the intent resource,
- records `TaskCreated` with `intent: Some(intent)`,
- returns the task id.

This intentionally changes the core API. Existing callers must declare an
intent before creating a task.

Task lifecycle methods:

- continue to find the resource through the task record,
- include the task intent id in task lifecycle events,
- do not mutate the intent record in V0.

`delegate_task`:

- continues to derive a task-scoped action capability,
- lifecycle and capability-derived events include the task intent id where the
  event already has a task.

## Error Handling

Add:

```rust
IntentStoreFull
IntentNotFound
IntentAgentMismatch
```

Use existing errors for the rest:

- `ResourceNotFound` for missing resources,
- `CapabilityNotFound`, `CapabilityRevoked`, `AgentMismatch`,
  `ResourceMismatch`, and `OperationDenied` for capability failures,
- `EventLogFull` when intent declaration or task creation cannot be recorded,
- `TaskStoreFull` when a task cannot be allocated.

Failed intent declarations must leave intent state, event state, and id counters
unchanged. Failed task creation must leave task state, event state, and task id
counters unchanged.

## Determinism And Boundaries

The intent store uses fixed-capacity arrays, typed IDs, enums, and explicit
errors. It does not use heap allocation, strings, model calls, prompt parsing,
filesystem access, networking, randomness, timers, threads, or host I/O.

High-level planning remains in the supervisor. The kernel records structured
intent metadata and enforces capability checks; it does not interpret natural
language or decide strategy.

## Tests

Implementation must start with failing tests.

Core tests:

- declaring an action intent stores the record and records `IntentDeclared`,
- declaring an intent requires the capability for that intent kind,
- intent declaration returns `IntentStoreFull` without mutation when capacity is
  exhausted,
- intent declaration returns `EventLogFull` without mutation when the event log
  is full,
- creating a task from an intent binds `Task.intent` and records the intent id,
- creating a task with another agent's intent returns `IntentAgentMismatch`
  without mutation,
- task lifecycle events include the task intent id.

Facade tests:

- `sys_declare_intent` records an intent and exposes it through `intents()`,
- `sys_create_task` accepts an intent id and creates an intent-bound task.

Supervisor test:

- host flow declares an action intent before task creation and prints
  `intent_declared`.

Full verification:

```bash
PATH="$HOME/.cargo/bin:$PATH" rustup run nightly cargo fmt --check
PATH="$HOME/.cargo/bin:$PATH" RUSTC="$(rustup which rustc --toolchain nightly)" rustup run nightly cargo test --workspace
PATH="$HOME/.cargo/bin:$PATH" RUSTC="$(rustup which rustc --toolchain nightly)" rustup run nightly cargo run -p agent-supervisor
scripts/run-qemu.sh
```

## Compatibility Impact

This design changes the `KernelCore`, `AgentKernel`, and `BootedKernel`
generic parameter lists by adding intent capacity.

This design changes task creation APIs from resource-based task creation to
intent-based task creation. Existing tests, supervisor flow, and README traces
must be updated to declare an intent before creating a task.

This design extends public `Event` and `Task` structs and adds public
`Intent`-related types.

## Deferred Work

V0 does not include:

- natural-language intent payloads,
- prompt storage,
- policy engines,
- intent status mutation,
- intent cancellation,
- multiple tasks per intent constraints,
- intent priority,
- scheduler ordering by intent risk or priority,
- replay engine implementation.
