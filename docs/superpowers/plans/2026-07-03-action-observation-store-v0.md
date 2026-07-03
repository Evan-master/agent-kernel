# Action Observation Store V0 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add deterministic, queryable kernel records for authorized observations and executed actions.

**Architecture:** `agent-kernel-core` gains fixed-capacity action and observation stores, and observation/action/verification requests mutate those stores atomically with event recording. The generic `authorize(...)` event shortcut is removed so callers cannot bypass the new stores; the facade keeps syscall-style methods and exposes read-only record inspection.

**Tech Stack:** Rust nightly, no_std core and facade crates, fixed-capacity arrays, Cargo workspace tests, QEMU BIOS boot script.

---

## File Structure

- Create `crates/agent-kernel-core/src/action.rs` for `ActionRecord` and `ActionStatus`.
- Create `crates/agent-kernel-core/src/action_store.rs` for action lookup, execution, verification-request transitions, and read-only inspection.
- Create `crates/agent-kernel-core/src/observation.rs` for `ObservationRecord`.
- Create `crates/agent-kernel-core/src/observation_store.rs` for observation allocation, event recording, and read-only inspection.
- Modify `crates/agent-kernel-core/src/id.rs` for `ObservationId`.
- Modify `crates/agent-kernel-core/src/core.rs` for generic capacities and base state initialization.
- Modify `crates/agent-kernel-core/src/event.rs` for `Event.observation`.
- Modify all core modules with `KernelCore<...>` impl headers to include `ACTIONS` and `OBSERVATIONS`.
- Modify all event construction sites to set `observation: None` unless creating an observation event.
- Modify `crates/agent-kernel-core/src/error.rs` and `lib.rs` for public errors and exports.
- Create `crates/agent-kernel-core/tests/action_observation_store.rs` for red/green core store behavior.
- Modify existing core tests that currently call `authorize(...)` or use old `KernelCore` type arity.
- Modify `crates/agent-kernel/src/lib.rs` and `scheduler.rs` for new `AgentKernel` type arity, `sys_observe`, and read-only record accessors.
- Modify `crates/agent-kernel/tests/kernel_facade.rs`, `intent_lifecycle.rs`, and `capability_lifecycle.rs` for facade visibility and generic arity.
- Modify `crates/agent-kernel-boot/src/lib.rs`, `crates/agent-kernel-boot/tests/boot_flow.rs`, and `crates/agent-kernel-x86_64/src/main.rs` for capacity arity.
- Modify `crates/agent-supervisor/src/main.rs` only if type aliases or output checks need capacity adjustments; expected output remains unchanged.
- Modify `README.md` to mention action and observation records in current scope/behavior.

## Task 1: Core Action And Observation Red Tests

**Files:**
- Create: `crates/agent-kernel-core/tests/action_observation_store.rs`

- [ ] **Step 1: Create focused red tests**

Create `crates/agent-kernel-core/tests/action_observation_store.rs`:

```rust
use agent_kernel_core::{
    ActionId, ActionStatus, AgentId, EventKind, KernelCore, KernelError, ObservationId,
    Operation, OperationSet, ResourceKind,
};

type TestCore = KernelCore<4, 4, 16, 4, 4, 0, 0, 0>;

#[test]
fn observe_records_observation_and_event() {
    let mut core = TestCore::new();
    let agent = AgentId::new(1);
    let resource = core
        .register_resource(ResourceKind::Workspace, None)
        .expect("resource should fit");
    let capability = core
        .grant_capability(agent, resource, OperationSet::only(Operation::Observe))
        .expect("capability should fit");

    let event = core
        .observe(agent, capability, resource)
        .expect("observation should record");

    assert_eq!(core.observations().len(), 1);
    assert_eq!(core.observations()[0].id, ObservationId::new(1));
    assert_eq!(core.observations()[0].agent, agent);
    assert_eq!(core.observations()[0].resource, resource);
    assert_eq!(core.observations()[0].capability, capability);
    assert_eq!(event.kind, EventKind::Observation);
    assert_eq!(event.observation, Some(ObservationId::new(1)));
    assert_eq!(event.resource, Some(resource));
    assert_eq!(event.capability, Some(capability));
    assert_eq!(event.operation, Some(Operation::Observe));
    assert_eq!(core.events()[1].observation, Some(ObservationId::new(1)));
}

#[test]
fn observe_store_full_leaves_events_unchanged() {
    let mut core = KernelCore::<1, 1, 4, 1, 0, 0, 0, 0>::new();
    let agent = AgentId::new(2);
    let resource = core
        .register_resource(ResourceKind::Workspace, None)
        .expect("resource should fit");
    let capability = core
        .grant_capability(agent, resource, OperationSet::only(Operation::Observe))
        .expect("capability should fit");
    let events_after_grant = core.events().len();

    let result = core.observe(agent, capability, resource);

    assert_eq!(result, Err(KernelError::ObservationStoreFull));
    assert!(core.observations().is_empty());
    assert_eq!(core.events().len(), events_after_grant);
}

#[test]
fn observe_event_log_full_leaves_observations_unchanged() {
    let mut core = KernelCore::<1, 1, 1, 1, 1, 0, 0, 0>::new();
    let agent = AgentId::new(3);
    let resource = core
        .register_resource(ResourceKind::Workspace, None)
        .expect("resource should fit");
    let capability = core
        .grant_capability(agent, resource, OperationSet::only(Operation::Observe))
        .expect("grant should consume only event slot");

    let result = core.observe(agent, capability, resource);

    assert_eq!(result, Err(KernelError::EventLogFull));
    assert!(core.observations().is_empty());
    assert_eq!(core.events().len(), 1);
}

#[test]
fn act_records_action_and_event() {
    let mut core = TestCore::new();
    let agent = AgentId::new(4);
    let resource = core
        .register_resource(ResourceKind::Workspace, None)
        .expect("resource should fit");
    let capability = core
        .grant_capability(agent, resource, OperationSet::only(Operation::Act))
        .expect("capability should fit");
    let action = ActionId::new(9);

    let event = core
        .act(agent, capability, action, resource)
        .expect("action should record");

    assert_eq!(core.actions().len(), 1);
    assert_eq!(core.actions()[0].id, action);
    assert_eq!(core.actions()[0].agent, agent);
    assert_eq!(core.actions()[0].resource, resource);
    assert_eq!(core.actions()[0].capability, capability);
    assert_eq!(core.actions()[0].status, ActionStatus::Executed);
    assert_eq!(event.kind, EventKind::ActionExecuted);
    assert_eq!(event.action, Some(action));
    assert_eq!(event.observation, None);
}

#[test]
fn act_rejects_duplicate_action_without_event() {
    let mut core = TestCore::new();
    let agent = AgentId::new(5);
    let resource = core
        .register_resource(ResourceKind::Workspace, None)
        .expect("resource should fit");
    let capability = core
        .grant_capability(agent, resource, OperationSet::only(Operation::Act))
        .expect("capability should fit");
    let action = ActionId::new(10);
    core.act(agent, capability, action, resource)
        .expect("first action should record");
    let events_after_first = core.events().len();

    let result = core.act(agent, capability, action, resource);

    assert_eq!(result, Err(KernelError::ActionAlreadyExists));
    assert_eq!(core.actions().len(), 1);
    assert_eq!(core.actions()[0].status, ActionStatus::Executed);
    assert_eq!(core.events().len(), events_after_first);
}

#[test]
fn act_store_full_leaves_events_unchanged() {
    let mut core = KernelCore::<1, 1, 4, 0, 1, 0, 0, 0>::new();
    let agent = AgentId::new(6);
    let resource = core
        .register_resource(ResourceKind::Workspace, None)
        .expect("resource should fit");
    let capability = core
        .grant_capability(agent, resource, OperationSet::only(Operation::Act))
        .expect("capability should fit");
    let events_after_grant = core.events().len();

    let result = core.act(agent, capability, ActionId::new(11), resource);

    assert_eq!(result, Err(KernelError::ActionStoreFull));
    assert!(core.actions().is_empty());
    assert_eq!(core.events().len(), events_after_grant);
}

#[test]
fn act_event_log_full_leaves_actions_unchanged() {
    let mut core = KernelCore::<1, 1, 1, 1, 1, 0, 0, 0>::new();
    let agent = AgentId::new(7);
    let resource = core
        .register_resource(ResourceKind::Workspace, None)
        .expect("resource should fit");
    let capability = core
        .grant_capability(agent, resource, OperationSet::only(Operation::Act))
        .expect("grant should consume only event slot");

    let result = core.act(agent, capability, ActionId::new(12), resource);

    assert_eq!(result, Err(KernelError::EventLogFull));
    assert!(core.actions().is_empty());
    assert_eq!(core.events().len(), 1);
}

#[test]
fn verify_existing_action_updates_status_and_event() {
    let mut core = TestCore::new();
    let agent = AgentId::new(8);
    let resource = core
        .register_resource(ResourceKind::Workspace, None)
        .expect("resource should fit");
    let capability = core
        .grant_capability(
            agent,
            resource,
            OperationSet::empty()
                .with(Operation::Act)
                .with(Operation::Verify),
        )
        .expect("capability should fit");
    let action = ActionId::new(13);
    core.act(agent, capability, action, resource)
        .expect("action should record");

    let event = core
        .verify(agent, capability, action, resource)
        .expect("verification should record");

    assert_eq!(core.actions()[0].status, ActionStatus::VerificationRequested);
    assert_eq!(event.kind, EventKind::VerificationRequested);
    assert_eq!(event.action, Some(action));
    assert_eq!(event.resource, Some(resource));
}

#[test]
fn verify_missing_action_leaves_events_unchanged() {
    let mut core = TestCore::new();
    let agent = AgentId::new(9);
    let resource = core
        .register_resource(ResourceKind::Workspace, None)
        .expect("resource should fit");
    let capability = core
        .grant_capability(agent, resource, OperationSet::only(Operation::Verify))
        .expect("capability should fit");
    let events_after_grant = core.events().len();

    let result = core.verify(agent, capability, ActionId::new(14), resource);

    assert_eq!(result, Err(KernelError::ActionNotFound));
    assert_eq!(core.events().len(), events_after_grant);
}

#[test]
fn verify_rejects_action_resource_mismatch_without_status_change() {
    let mut core = KernelCore::<2, 2, 8, 2, 1, 0, 0, 0>::new();
    let agent = AgentId::new(10);
    let first_resource = core
        .register_resource(ResourceKind::Workspace, None)
        .expect("first resource should fit");
    let second_resource = core
        .register_resource(ResourceKind::Service, None)
        .expect("second resource should fit");
    let act_capability = core
        .grant_capability(agent, first_resource, OperationSet::only(Operation::Act))
        .expect("act capability should fit");
    let verify_capability = core
        .grant_capability(
            agent,
            second_resource,
            OperationSet::only(Operation::Verify),
        )
        .expect("verify capability should fit");
    let action = ActionId::new(15);
    core.act(agent, act_capability, action, first_resource)
        .expect("action should record");
    let events_after_action = core.events().len();

    let result = core.verify(agent, verify_capability, action, second_resource);

    assert_eq!(result, Err(KernelError::ActionResourceMismatch));
    assert_eq!(core.actions()[0].status, ActionStatus::Executed);
    assert_eq!(core.events().len(), events_after_action);
}

#[test]
fn verify_rejects_repeated_verification_without_event() {
    let mut core = TestCore::new();
    let agent = AgentId::new(11);
    let resource = core
        .register_resource(ResourceKind::Workspace, None)
        .expect("resource should fit");
    let capability = core
        .grant_capability(
            agent,
            resource,
            OperationSet::empty()
                .with(Operation::Act)
                .with(Operation::Verify),
        )
        .expect("capability should fit");
    let action = ActionId::new(16);
    core.act(agent, capability, action, resource)
        .expect("action should record");
    core.verify(agent, capability, action, resource)
        .expect("first verification should record");
    let events_after_verify = core.events().len();

    let result = core.verify(agent, capability, action, resource);

    assert_eq!(result, Err(KernelError::ActionStatusMismatch));
    assert_eq!(core.actions()[0].status, ActionStatus::VerificationRequested);
    assert_eq!(core.events().len(), events_after_verify);
}

#[test]
fn verify_event_log_full_leaves_action_status_executed() {
    let mut core = KernelCore::<1, 1, 2, 1, 1, 0, 0, 0>::new();
    let agent = AgentId::new(12);
    let resource = core
        .register_resource(ResourceKind::Workspace, None)
        .expect("resource should fit");
    let capability = core
        .grant_capability(
            agent,
            resource,
            OperationSet::empty()
                .with(Operation::Act)
                .with(Operation::Verify),
        )
        .expect("grant should consume first event");
    let action = ActionId::new(17);
    core.act(agent, capability, action, resource)
        .expect("act should consume second event");

    let result = core.verify(agent, capability, action, resource);

    assert_eq!(result, Err(KernelError::EventLogFull));
    assert_eq!(core.actions()[0].status, ActionStatus::Executed);
    assert_eq!(core.events().len(), 2);
}
```

- [ ] **Step 2: Run red core test**

Run:

```bash
PATH="$HOME/.cargo/bin:$PATH" RUSTC="$(rustup which rustc --toolchain nightly)" rustup run nightly cargo test -p agent-kernel-core --test action_observation_store
```

Expected: compile failures for missing `ActionStatus`, `ObservationId`, `observe`, `actions`, `observations`, new `KernelError` variants, and new `KernelCore` generic arity.

## Task 2: Core Data Model And Event Shape

**Files:**
- Modify: `crates/agent-kernel-core/src/id.rs`
- Create: `crates/agent-kernel-core/src/action.rs`
- Create: `crates/agent-kernel-core/src/observation.rs`
- Modify: `crates/agent-kernel-core/src/error.rs`
- Modify: `crates/agent-kernel-core/src/event.rs`
- Modify: `crates/agent-kernel-core/src/core.rs`
- Modify: `crates/agent-kernel-core/src/lib.rs`

- [ ] **Step 1: Add `ObservationId`**

In `crates/agent-kernel-core/src/id.rs`, add this block after `ActionId`:

```rust
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct ObservationId(u64);

impl ObservationId {
    pub const fn new(raw: u64) -> Self {
        Self(raw)
    }

    pub const fn raw(self) -> u64 {
        self.0
    }
}
```

- [ ] **Step 2: Add action records**

Create `crates/agent-kernel-core/src/action.rs`:

```rust
//! Kernel-visible action execution records.
//!
//! This module belongs to `agent-kernel-core`. It defines fixed-size action
//! facts that later verifier and rollback primitives can inspect without
//! replaying the entire event log.

use crate::{ActionId, AgentId, CapabilityId, ResourceId};

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

impl ActionRecord {
    pub(crate) const fn empty() -> Self {
        Self {
            id: ActionId::new(0),
            agent: AgentId::new(0),
            resource: ResourceId::new(0),
            capability: CapabilityId::new(0),
            status: ActionStatus::Executed,
        }
    }
}
```

- [ ] **Step 3: Add observation records**

Create `crates/agent-kernel-core/src/observation.rs`:

```rust
//! Kernel-visible observation records.
//!
//! This module belongs to `agent-kernel-core`. It defines fixed-size
//! observation facts that can be replayed and inspected without storing host
//! observation payloads in kernel space.

use crate::{AgentId, CapabilityId, ObservationId, ResourceId};

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct ObservationRecord {
    pub id: ObservationId,
    pub agent: AgentId,
    pub resource: ResourceId,
    pub capability: CapabilityId,
}

impl ObservationRecord {
    pub(crate) const fn empty() -> Self {
        Self {
            id: ObservationId::new(0),
            agent: AgentId::new(0),
            resource: ResourceId::new(0),
            capability: CapabilityId::new(0),
        }
    }
}
```

- [ ] **Step 4: Add error values**

In `crates/agent-kernel-core/src/error.rs`, add these variants near the other
store and status errors:

```rust
ActionStoreFull,
ObservationStoreFull,
ActionAlreadyExists,
ActionNotFound,
ActionResourceMismatch,
ActionStatusMismatch,
```

- [ ] **Step 5: Add event observation field**

In `crates/agent-kernel-core/src/event.rs`, import `ObservationId`, then add
the field after `action`:

```rust
pub observation: Option<ObservationId>,
```

Set the empty event value:

```rust
observation: None,
```

- [ ] **Step 6: Extend `KernelCore` state**

In `crates/agent-kernel-core/src/core.rs`, import `ActionRecord` and
`ObservationRecord`:

```rust
use crate::{
    ActionId, ActionRecord, AgentId, Capability, CapabilityId, CheckpointId, Event, EventKind,
    Intent, KernelError, ObservationRecord, Operation, OperationSet, Resource, ResourceId,
    RunQueueEntry, Task,
};
```

Change the generic parameter list everywhere in `core.rs` from:

```rust
const RESOURCES: usize,
const CAPS: usize,
const EVENTS: usize,
const INTENTS: usize,
const TASKS: usize,
const RUN_QUEUE: usize,
```

to:

```rust
const RESOURCES: usize,
const CAPS: usize,
const EVENTS: usize,
const ACTIONS: usize,
const OBSERVATIONS: usize,
const INTENTS: usize,
const TASKS: usize,
const RUN_QUEUE: usize,
```

Add fields to `KernelCore` after `events`:

```rust
pub(crate) actions: [ActionRecord; ACTIONS],
pub(crate) observations: [ObservationRecord; OBSERVATIONS],
```

Add lengths and id counter:

```rust
pub(crate) action_len: usize,
pub(crate) observation_len: usize,
pub(crate) next_observation: u64,
```

Initialize them in `new()`:

```rust
actions: [ActionRecord::empty(); ACTIONS],
observations: [ObservationRecord::empty(); OBSERVATIONS],
action_len: 0,
observation_len: 0,
next_observation: 1,
```

Keep `next_sequence` unchanged.

- [ ] **Step 7: Update `resource_event`**

In `resource_event(...)`, set:

```rust
observation: None,
```

This helper remains for checkpoint and rollback events only after Task 3 removes
the public generic `authorize(...)` path.

- [ ] **Step 8: Register modules and exports**

In `crates/agent-kernel-core/src/lib.rs`, add modules:

```rust
mod action;
mod observation;
```

Update exports:

```rust
pub use action::{ActionRecord, ActionStatus};
pub use id::{
    ActionId, AgentId, CapabilityId, CheckpointId, IntentId, ObservationId, ResourceId, TaskId,
};
pub use observation::ObservationRecord;
```

- [ ] **Step 9: Run red test again**

Run:

```bash
PATH="$HOME/.cargo/bin:$PATH" RUSTC="$(rustup which rustc --toolchain nightly)" rustup run nightly cargo test -p agent-kernel-core --test action_observation_store
```

Expected: compile progresses farther and fails because `observe`, `actions`,
`observations`, and generic arity updates in other modules are not complete.

## Task 3: Propagate Core Generic Arity And Event Field

**Files:**
- Modify every `crates/agent-kernel-core/src/*.rs` file with a `KernelCore<...>` impl.
- Modify all core tests using `KernelCore<...>`.
- Modify all event construction sites.

- [ ] **Step 1: Find all core generic headers**

Run:

```bash
rg -n "KernelCore<|const INTENTS|const TASKS|const RUN_QUEUE" crates/agent-kernel-core/src crates/agent-kernel-core/tests -g '*.rs'
```

Expected: matches in `authorization.rs`, `capability_store.rs`, `core.rs`,
`event_log.rs`, `intent_event.rs`, `intent_store.rs`, `lookup.rs`,
`resource_store.rs`, `scheduler.rs`, `task_event.rs`, `task_store.rs`, and
core tests.

- [ ] **Step 2: Update source impl headers**

For every `impl` over `KernelCore`, use this generic list:

```rust
impl<
        const RESOURCES: usize,
        const CAPS: usize,
        const EVENTS: usize,
        const ACTIONS: usize,
        const OBSERVATIONS: usize,
        const INTENTS: usize,
        const TASKS: usize,
        const RUN_QUEUE: usize,
    > KernelCore<RESOURCES, CAPS, EVENTS, ACTIONS, OBSERVATIONS, INTENTS, TASKS, RUN_QUEUE>
```

For helper signatures that accept a generic core value, use:

```rust
KernelCore<RESOURCES, CAPS, EVENTS, ACTIONS, OBSERVATIONS, INTENTS, TASKS, RUN_QUEUE>
```

- [ ] **Step 3: Update test type aliases**

Replace aliases with two new capacities after `EVENTS`. Use these concrete
values:

```rust
// crates/agent-kernel-core/tests/kernel_core.rs
type TestCore = KernelCore<4, 4, 16, 4, 4, 0, 4, 4>;

// crates/agent-kernel-core/tests/capability_lifecycle.rs
type TestCore = KernelCore<4, 8, 32, 2, 2, 6, 6, 4>;

// crates/agent-kernel-core/tests/capability_revocation.rs
type TestCore = KernelCore<4, 8, 32, 2, 2, 6, 6, 4>;

// crates/agent-kernel-core/tests/delegated_capability.rs
type TestCore = KernelCore<4, 8, 32, 4, 2, 6, 6, 4>;

// crates/agent-kernel-core/tests/intent_store.rs
type TestCore = KernelCore<4, 8, 64, 2, 2, 4, 6, 4>;

// crates/agent-kernel-core/tests/scheduler.rs
type TestCore = KernelCore<4, 6, 32, 4, 2, 6, 6, 4>;

// crates/agent-kernel-core/tests/task_authority.rs
type TestCore = KernelCore<4, 4, 24, 2, 2, 4, 4, 4>;

// crates/agent-kernel-core/tests/task_lifecycle.rs
type TestCore = KernelCore<4, 4, 16, 2, 2, 4, 4, 4>;
```

Update inline `KernelCore::<...>` uses by inserting action and observation
capacities after `EVENTS`. For tests that never call `act` or `observe`, use
`1, 1`. For tests that call `act`, use at least `1, 1`.

- [ ] **Step 4: Update all event constructors**

Run:

```bash
rg -n "Event \\{" crates/agent-kernel-core/src -g '*.rs'
```

In every `Event { ... }` literal outside observation events, add:

```rust
observation: None,
```

Expected construction sites include:

- `core.rs`,
- `capability_store.rs`,
- `intent_event.rs`,
- `intent_store.rs`,
- `scheduler.rs`,
- `task_event.rs`.

- [ ] **Step 5: Run core compile check**

Run:

```bash
PATH="$HOME/.cargo/bin:$PATH" RUSTC="$(rustup which rustc --toolchain nightly)" rustup run nightly cargo test -p agent-kernel-core --test action_observation_store
```

Expected: remaining failures are missing store methods and removed `authorize`
test call updates.

## Task 4: Implement Observation Store

**Files:**
- Create: `crates/agent-kernel-core/src/observation_store.rs`
- Modify: `crates/agent-kernel-core/src/lib.rs`
- Modify: `crates/agent-kernel-core/src/core.rs`

- [ ] **Step 1: Create observation store module**

Create `crates/agent-kernel-core/src/observation_store.rs`:

```rust
//! Fixed-capacity observation store.
//!
//! This module belongs to `agent-kernel-core`. It records authorized
//! observations as queryable kernel facts while keeping host observation
//! payloads outside kernel space.

use crate::{
    AgentId, CapabilityId, Event, EventKind, KernelCore, KernelError, ObservationId,
    ObservationRecord, Operation, OperationSet, ResourceId, VerificationRequirement,
};

impl<
        const RESOURCES: usize,
        const CAPS: usize,
        const EVENTS: usize,
        const ACTIONS: usize,
        const OBSERVATIONS: usize,
        const INTENTS: usize,
        const TASKS: usize,
        const RUN_QUEUE: usize,
    > KernelCore<RESOURCES, CAPS, EVENTS, ACTIONS, OBSERVATIONS, INTENTS, TASKS, RUN_QUEUE>
{
    pub fn observe(
        &mut self,
        agent: AgentId,
        capability: CapabilityId,
        resource: ResourceId,
    ) -> Result<Event, KernelError> {
        self.ensure_authorized(agent, capability, resource, Operation::Observe)?;
        if self.observation_len >= OBSERVATIONS {
            return Err(KernelError::ObservationStoreFull);
        }
        self.ensure_event_slots(1)?;

        let observation = ObservationId::new(self.next_observation);
        self.next_observation += 1;
        self.observations[self.observation_len] = ObservationRecord {
            id: observation,
            agent,
            resource,
            capability,
        };
        self.observation_len += 1;

        self.record(Event {
            sequence: self.next_sequence,
            agent,
            kind: EventKind::Observation,
            resource: Some(resource),
            capability: Some(capability),
            source_capability: None,
            intent: None,
            intent_kind: None,
            action: None,
            observation: Some(observation),
            operation: Some(Operation::Observe),
            operations: OperationSet::empty(),
            verification: VerificationRequirement::Optional,
            checkpoint: None,
            task: None,
            target_agent: None,
        })
    }

    pub fn observations(&self) -> &[ObservationRecord] {
        &self.observations[..self.observation_len]
    }
}
```

- [ ] **Step 2: Register observation store**

In `crates/agent-kernel-core/src/lib.rs`, add:

```rust
mod observation_store;
```

- [ ] **Step 3: Remove generic observe path from `core.rs`**

Delete `KernelCore::authorize(...)` and `event_kind(...)` from
`crates/agent-kernel-core/src/core.rs`.

Keep `resource_event(...)` because checkpoint and rollback still use it.

- [ ] **Step 4: Run observation-focused tests**

Run:

```bash
PATH="$HOME/.cargo/bin:$PATH" RUSTC="$(rustup which rustc --toolchain nightly)" rustup run nightly cargo test -p agent-kernel-core --test action_observation_store observe
```

Expected: observation tests pass; action and verification tests still fail until Task 5.

## Task 5: Implement Action Store And Verification Status

**Files:**
- Create: `crates/agent-kernel-core/src/action_store.rs`
- Modify: `crates/agent-kernel-core/src/lib.rs`
- Modify: `crates/agent-kernel-core/src/core.rs`

- [ ] **Step 1: Create action store module**

Create `crates/agent-kernel-core/src/action_store.rs`:

```rust
//! Fixed-capacity action store and verification-request state.
//!
//! This module belongs to `agent-kernel-core`. It records authorized action
//! execution as deterministic kernel state and moves actions into a
//! verification-requested state without executing verifier policy.

use crate::{
    ActionId, ActionRecord, ActionStatus, AgentId, CapabilityId, Event, EventKind, KernelCore,
    KernelError, Operation, OperationSet, ResourceId, VerificationRequirement,
};

impl<
        const RESOURCES: usize,
        const CAPS: usize,
        const EVENTS: usize,
        const ACTIONS: usize,
        const OBSERVATIONS: usize,
        const INTENTS: usize,
        const TASKS: usize,
        const RUN_QUEUE: usize,
    > KernelCore<RESOURCES, CAPS, EVENTS, ACTIONS, OBSERVATIONS, INTENTS, TASKS, RUN_QUEUE>
{
    pub fn act(
        &mut self,
        agent: AgentId,
        capability: CapabilityId,
        action: ActionId,
        resource: ResourceId,
    ) -> Result<Event, KernelError> {
        self.ensure_authorized(agent, capability, resource, Operation::Act)?;
        if self.find_action(action).is_ok() {
            return Err(KernelError::ActionAlreadyExists);
        }
        if self.action_len >= ACTIONS {
            return Err(KernelError::ActionStoreFull);
        }
        self.ensure_event_slots(1)?;

        self.actions[self.action_len] = ActionRecord {
            id: action,
            agent,
            resource,
            capability,
            status: ActionStatus::Executed,
        };
        self.action_len += 1;

        self.record(Event {
            sequence: self.next_sequence,
            agent,
            kind: EventKind::ActionExecuted,
            resource: Some(resource),
            capability: Some(capability),
            source_capability: None,
            intent: None,
            intent_kind: None,
            action: Some(action),
            observation: None,
            operation: Some(Operation::Act),
            operations: OperationSet::empty(),
            verification: VerificationRequirement::Optional,
            checkpoint: None,
            task: None,
            target_agent: None,
        })
    }

    pub fn verify(
        &mut self,
        agent: AgentId,
        capability: CapabilityId,
        action: ActionId,
        resource: ResourceId,
    ) -> Result<Event, KernelError> {
        self.ensure_authorized(agent, capability, resource, Operation::Verify)?;
        let current = self.find_action(action)?;
        if current.resource != resource {
            return Err(KernelError::ActionResourceMismatch);
        }
        if current.status != ActionStatus::Executed {
            return Err(KernelError::ActionStatusMismatch);
        }
        self.ensure_event_slots(1)?;

        self.find_action_mut(action)?.status = ActionStatus::VerificationRequested;
        self.record(Event {
            sequence: self.next_sequence,
            agent,
            kind: EventKind::VerificationRequested,
            resource: Some(resource),
            capability: Some(capability),
            source_capability: None,
            intent: None,
            intent_kind: None,
            action: Some(action),
            observation: None,
            operation: Some(Operation::Verify),
            operations: OperationSet::empty(),
            verification: VerificationRequirement::Optional,
            checkpoint: None,
            task: None,
            target_agent: None,
        })
    }

    pub fn actions(&self) -> &[ActionRecord] {
        &self.actions[..self.action_len]
    }

    pub(crate) fn find_action(&self, id: ActionId) -> Result<ActionRecord, KernelError> {
        self.actions()
            .iter()
            .find(|action| action.id == id)
            .copied()
            .ok_or(KernelError::ActionNotFound)
    }

    fn find_action_mut(&mut self, id: ActionId) -> Result<&mut ActionRecord, KernelError> {
        self.actions[..self.action_len]
            .iter_mut()
            .find(|action| action.id == id)
            .ok_or(KernelError::ActionNotFound)
    }
}
```

- [ ] **Step 2: Register action store**

In `crates/agent-kernel-core/src/lib.rs`, add:

```rust
mod action_store;
```

- [ ] **Step 3: Remove old action methods from `core.rs`**

Delete `KernelCore::act(...)` and `KernelCore::verify(...)` from
`crates/agent-kernel-core/src/core.rs`.

Remove `ActionId` from `core.rs` imports if it is no longer used by
`resource_event(...)`. If `resource_event(...)` still accepts `action`, change
the helper signature to remove the action argument and set `action: None`.

- [ ] **Step 4: Run focused store tests**

Run:

```bash
PATH="$HOME/.cargo/bin:$PATH" RUSTC="$(rustup which rustc --toolchain nightly)" rustup run nightly cargo test -p agent-kernel-core --test action_observation_store
```

Expected: all tests in `action_observation_store.rs` pass.

## Task 6: Update Existing Core Tests And Remove `authorize(...)` Callers

**Files:**
- Modify: `crates/agent-kernel-core/tests/kernel_core.rs`
- Modify: `crates/agent-kernel-core/tests/capability_lifecycle.rs`
- Modify any other core test found by the search command.

- [ ] **Step 1: Find remaining `authorize(...)` callers**

Run:

```bash
rg -n "\\.authorize\\(" crates -g '*.rs'
```

Expected before edits: matches only in tests and facade implementation. After
this task, no matches remain.

- [ ] **Step 2: Update observation test**

In `crates/agent-kernel-core/tests/kernel_core.rs`, replace
`observes_resource_when_capability_allows_observe` with:

```rust
#[test]
fn observes_resource_when_capability_allows_observe() {
    let mut core = TestCore::new();
    let agent = AgentId::new(7);
    let resource = core
        .register_resource(ResourceKind::Workspace, None)
        .expect("resource should fit");
    let capability = core
        .grant_capability(agent, resource, OperationSet::only(Operation::Observe))
        .expect("capability should fit");

    let event = core
        .observe(agent, capability, resource)
        .expect("observe should be authorized");

    assert_eq!(event.agent, agent);
    assert_eq!(event.resource, Some(resource));
    assert_eq!(event.kind, EventKind::Observation);
    assert_eq!(core.observations().len(), 1);
    assert_eq!(core.events().len(), 2);
    assert_eq!(core.events()[0].kind, EventKind::CapabilityGranted);
    assert_eq!(core.events()[1].kind, EventKind::Observation);
}
```

- [ ] **Step 3: Replace generic authorization denial test**

In `crates/agent-kernel-core/tests/kernel_core.rs`, replace
`denies_action_when_capability_does_not_include_operation` with an `act(...)`
call:

```rust
#[test]
fn denies_action_when_capability_does_not_include_operation() {
    let mut core = TestCore::new();
    let agent = AgentId::new(1);
    let resource = core
        .register_resource(ResourceKind::Workspace, None)
        .expect("resource should fit");
    let capability = core
        .grant_capability(agent, resource, OperationSet::only(Operation::Observe))
        .expect("capability should fit");

    let result = core.act(agent, capability, ActionId::new(1), resource);

    assert_eq!(result, Err(KernelError::OperationDenied));
    assert_eq!(core.actions().len(), 0);
    assert_eq!(core.events().len(), 1);
    assert_eq!(core.events()[0].kind, EventKind::CapabilityGranted);
}
```

Add `KernelError` to the import list if missing.

- [ ] **Step 4: Replace revoked authorization test**

In `crates/agent-kernel-core/tests/kernel_core.rs`, replace the revoked
capability check with `observe(...)`:

```rust
assert_eq!(
    core.observe(agent, capability, resource),
    Err(KernelError::CapabilityRevoked)
);
```

Assert `core.observations().is_empty()`.

- [ ] **Step 5: Update action verification assertions**

In `action_and_verification_events_are_recorded_with_action_id`, add:

```rust
assert_eq!(core.actions().len(), 1);
assert_eq!(core.actions()[0].status, ActionStatus::VerificationRequested);
```

Add `ActionStatus` to the import list.

- [ ] **Step 6: Update capability lifecycle tests**

In `crates/agent-kernel-core/tests/capability_lifecycle.rs`:

Replace `core.authorize(agent, CapabilityId::new(1), resource, Operation::Observe)` with:

```rust
core.observe(agent, CapabilityId::new(1), resource)
```

Replace `core.authorize(agent, capability, resource, Operation::Observe)` with:

```rust
core.observe(agent, capability, resource)
```

Replace `core.authorize(agent, capability, resource, Operation::Act)` with:

```rust
core.act(agent, capability, ActionId::new(1), resource)
```

Add `ActionId` to the import list.

- [ ] **Step 7: Run core tests**

Run:

```bash
PATH="$HOME/.cargo/bin:$PATH" RUSTC="$(rustup which rustc --toolchain nightly)" rustup run nightly cargo test -p agent-kernel-core
```

Expected: all core tests pass.

## Task 7: Update Facade, Boot, Supervisor, And README

**Files:**
- Modify: `crates/agent-kernel/src/lib.rs`
- Modify: `crates/agent-kernel/src/scheduler.rs`
- Modify: `crates/agent-kernel/tests/capability_lifecycle.rs`
- Modify: `crates/agent-kernel/tests/intent_lifecycle.rs`
- Modify: `crates/agent-kernel/tests/kernel_facade.rs`
- Modify: `crates/agent-kernel-boot/src/lib.rs`
- Modify: `crates/agent-kernel-boot/tests/boot_flow.rs`
- Modify: `crates/agent-kernel-x86_64/src/main.rs`
- Modify: `crates/agent-supervisor/src/main.rs`
- Modify: `crates/agent-supervisor/tests/supervisor_flow.rs`
- Modify: `README.md`

- [ ] **Step 1: Update facade generics and imports**

In `crates/agent-kernel/src/lib.rs`, import record types:

```rust
ActionRecord, ActionStatus, ObservationRecord,
```

Change `AgentKernel` generics to include:

```rust
const ACTIONS: usize,
const OBSERVATIONS: usize,
```

between `EVENTS` and `INTENTS`, and update the core field:

```rust
pub(crate) core:
    KernelCore<RESOURCES, CAPS, EVENTS, ACTIONS, OBSERVATIONS, INTENTS, TASKS, RUN_QUEUE>,
```

Apply the same arity to every `impl AgentKernel<...>` block in `lib.rs` and
`scheduler.rs`.

- [ ] **Step 2: Update facade observe and accessors**

In `crates/agent-kernel/src/lib.rs`, change `sys_observe` to:

```rust
self.core.observe(agent, capability, resource)
```

Add read-only accessors:

```rust
pub fn actions(&self) -> &[ActionRecord] {
    self.core.actions()
}

pub fn observations(&self) -> &[ObservationRecord] {
    self.core.observations()
}
```

Remove unused imports after `cargo fmt`.

- [ ] **Step 3: Update facade test aliases**

Use these aliases:

```rust
// crates/agent-kernel/tests/capability_lifecycle.rs
type TestKernel = AgentKernel<2, 2, 4, 1, 1, 0, 1, 1>;

// crates/agent-kernel/tests/intent_lifecycle.rs
type TestKernel = AgentKernel<4, 4, 16, 4, 4, 4, 4, 4>;

// crates/agent-kernel/tests/kernel_facade.rs
type TestKernel = AgentKernel<4, 6, 64, 8, 8, 8, 8, 4>;
```

- [ ] **Step 4: Add facade visibility assertions**

In `crates/agent-kernel/tests/kernel_facade.rs`, import `ActionStatus`.

In `observe_syscall_records_observation_event`, add:

```rust
assert_eq!(kernel.observations().len(), 1);
assert_eq!(kernel.events()[1].observation, Some(kernel.observations()[0].id));
```

In `action_and_verify_syscalls_record_action_lifecycle`, add:

```rust
assert_eq!(kernel.actions().len(), 1);
assert_eq!(kernel.actions()[0].status, ActionStatus::VerificationRequested);
```

- [ ] **Step 5: Update boot arity**

In `crates/agent-kernel-boot/src/lib.rs`, add `ACTIONS` and `OBSERVATIONS` to
`BootedKernel` generics between `EVENTS` and `INTENTS`.

Update the field and `kernel()` return type:

```rust
AgentKernel<RESOURCES, CAPS, EVENTS, ACTIONS, OBSERVATIONS, INTENTS, TASKS, RUN_QUEUE>
```

- [ ] **Step 6: Update boot tests and x86 type aliases**

Use these aliases:

```rust
// crates/agent-kernel-boot/tests/boot_flow.rs
BootedKernel::<8, 8, 16, 4, 4, 0, 4, 4>

// crates/agent-kernel-x86_64/src/main.rs
BootedKernel::<8, 8, 16, 4, 4, 0, 4, 4>
```

QEMU serial output must remain:

```text
AGENT_KERNEL_QEMU_BOOT_OK
event[1] capability_granted
event[2] observation
event[3] action
event[4] verification
SUPERVISOR_HANDOFF_READY
```

- [ ] **Step 7: Update supervisor arity**

In `crates/agent-supervisor/src/main.rs`, change:

```rust
let mut kernel = AgentKernel::<8, 8, 32, 8, 8, 8>::new();
```

to:

```rust
let mut kernel = AgentKernel::<8, 8, 32, 8, 8, 8, 8, 8>::new();
```

The printed output remains unchanged.

- [ ] **Step 8: Update README**

In `README.md`, update the current scope bullet:

```text
- `agent-kernel-core`: no_std-friendly resource, capability, action, observation, intent store, task store, lifecycle, FIFO run queue, checkpoint, rollback, and event model.
```

In the current behavior list, change:

```text
4. Observe the resource.
5. Execute an action event with an `ActionId`.
```

to:

```text
4. Observe the resource and store an observation record.
5. Execute an action with an `ActionId` and store an action record.
```

Keep expected supervisor and QEMU output unchanged.

- [ ] **Step 9: Run facade, boot, and supervisor tests**

Run:

```bash
PATH="$HOME/.cargo/bin:$PATH" RUSTC="$(rustup which rustc --toolchain nightly)" rustup run nightly cargo test -p agent-kernel
PATH="$HOME/.cargo/bin:$PATH" RUSTC="$(rustup which rustc --toolchain nightly)" rustup run nightly cargo test -p agent-kernel-boot
PATH="$HOME/.cargo/bin:$PATH" RUSTC="$(rustup which rustc --toolchain nightly)" rustup run nightly cargo test -p agent-supervisor
```

Expected: all pass.

## Task 8: Full Verification And Publish

**Files:**
- All files changed by Tasks 1-7.

- [ ] **Step 1: Run formatting check**

Run:

```bash
PATH="$HOME/.cargo/bin:$PATH" rustup run nightly cargo fmt --check
```

Expected: command exits 0 with no output.

- [ ] **Step 2: Run workspace tests**

Run:

```bash
PATH="$HOME/.cargo/bin:$PATH" RUSTC="$(rustup which rustc --toolchain nightly)" rustup run nightly cargo test --workspace
```

Expected: all workspace tests pass.

- [ ] **Step 3: Run supervisor**

Run:

```bash
PATH="$HOME/.cargo/bin:$PATH" RUSTC="$(rustup which rustc --toolchain nightly)" rustup run nightly cargo run -p agent-supervisor
```

Expected output still includes:

```text
event[2] observation agent=1 resource=1
event[3] action agent=1 resource=1 action=1
event[4] verification agent=1 resource=1 action=1
event[17] intent_fulfilled agent=1 resource=1 intent=1
```

- [ ] **Step 4: Run QEMU**

Run:

```bash
scripts/run-qemu.sh
```

Expected serial output remains:

```text
AGENT_KERNEL_QEMU_BOOT_OK
event[1] capability_granted
event[2] observation
event[3] action
event[4] verification
SUPERVISOR_HANDOFF_READY
```

- [ ] **Step 5: Check no unsupported public generic authorization remains**

Run:

```bash
rg -n "\\.authorize\\(|pub fn authorize|event_kind\\(" crates -g '*.rs'
```

Expected: no matches.

- [ ] **Step 6: Check no_std boundary in new core modules**

Run:

```bash
rg -n "\\b(Vec|String|Box|HashMap|Rc|Arc|std::|println!|format!|thread|async|await|env::|fs::|net::)\\b" \
  crates/agent-kernel-core/src/action.rs \
  crates/agent-kernel-core/src/action_store.rs \
  crates/agent-kernel-core/src/observation.rs \
  crates/agent-kernel-core/src/observation_store.rs
```

Expected: no matches.

- [ ] **Step 7: Check file sizes**

Run:

```bash
wc -l crates/agent-kernel-core/src/*.rs crates/agent-kernel/src/*.rs crates/agent-supervisor/src/*.rs crates/agent-kernel-core/tests/*.rs crates/agent-kernel/tests/*.rs crates/agent-supervisor/tests/*.rs crates/agent-kernel-boot/src/*.rs crates/agent-kernel-boot/tests/*.rs crates/agent-kernel-x86_64/src/*.rs 2>/dev/null
```

Expected: no source file exceeds its hard limit. Existing test files may exceed
the 250-line soft limit if below the 500-line hard limit; report this as test
debt.

- [ ] **Step 8: Check diff hygiene**

Run:

```bash
git diff --check
git status --short
```

Expected: `git diff --check` exits 0. `git status --short` shows only files
needed for this implementation.

- [ ] **Step 9: Commit and push**

Run:

```bash
git add README.md crates docs/superpowers/plans/2026-07-03-action-observation-store-v0.md
git commit -m "feat: add action observation stores"
git push
```

Expected: `origin/main` receives the implementation commit.

## Self-Review

Spec coverage:

- Action records are covered by Tasks 1, 2, and 5.
- Observation records are covered by Tasks 1, 2, and 4.
- The generic `authorize(...)` bypass removal is covered by Tasks 4, 6, and 8.
- Event shape changes are covered by Tasks 2 and 3.
- Facade, boot, supervisor, and README compatibility are covered by Task 7.
- Full validation is covered by Task 8.

Placeholder scan:

- The plan uses concrete file paths, code snippets, commands, and expected
  outcomes.
- No task depends on an unspecified type or unnamed helper.

Type consistency:

- Generic order is consistently
  `RESOURCES, CAPS, EVENTS, ACTIONS, OBSERVATIONS, INTENTS, TASKS, RUN_QUEUE`.
- `ObservationId`, `ActionStatus`, `ActionRecord`, and `ObservationRecord`
  match the design spec names.
- `ActionStatus::VerificationRequested` is a request state, not a verifier
  result.
