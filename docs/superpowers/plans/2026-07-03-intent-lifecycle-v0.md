# Intent Lifecycle V0 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add deterministic lifecycle status to kernel intents and advance that status from existing task lifecycle operations.

**Architecture:** `agent-kernel-core` stores `IntentStatus` on each `Intent`, records task-driven intent lifecycle events, and keeps status mutation behind existing task syscalls. `agent-kernel` continues to expose read-only intent inspection, while `agent-supervisor` only prints the new intent lifecycle events.

**Tech Stack:** Rust nightly, no_std core and facade crates, fixed-capacity arrays, Cargo workspace tests, QEMU BIOS boot script.

---

## File Structure

- Modify `crates/agent-kernel-core/src/intent.rs` for `IntentStatus` and the `Intent.status` field.
- Modify `crates/agent-kernel-core/src/intent_store.rs` for initial status, mutable lookup, and status checks.
- Create `crates/agent-kernel-core/src/intent_event.rs` for intent lifecycle event construction.
- Modify `crates/agent-kernel-core/src/task_store.rs` for task-driven intent status transitions.
- Modify `crates/agent-kernel-core/src/error.rs`, `event.rs`, and `lib.rs` for new public types and events.
- Modify `crates/agent-kernel-core/tests/intent_store.rs` and task-related core tests for lifecycle assertions and shifted event ordering.
- Modify `crates/agent-kernel/tests/intent_lifecycle.rs` and `kernel_facade.rs` for facade visibility.
- Modify `crates/agent-supervisor/src/main.rs` and `tests/supervisor_flow.rs` for host trace output.
- Modify `crates/agent-kernel-x86_64/src/main.rs` for exhaustive event matching.
- Modify `README.md` for current behavior and expected supervisor output.

## Task 1: Core Intent Lifecycle Red Tests

**Files:**
- Modify: `crates/agent-kernel-core/tests/intent_store.rs`

- [ ] **Step 1: Add lifecycle imports**

Update the import block in `crates/agent-kernel-core/tests/intent_store.rs`:

```rust
use agent_kernel_core::{
    AgentId, EventKind, IntentId, IntentKind, IntentStatus, KernelCore, KernelError, Operation,
    OperationSet, ResourceKind, TaskStatus, VerificationRequirement,
};
```

- [ ] **Step 2: Extend declaration test**

In `declare_intent_records_typed_intent`, add:

```rust
assert_eq!(core.intents()[0].status, IntentStatus::Declared);
```

- [ ] **Step 3: Extend task creation test**

In `create_task_from_intent_binds_task_and_event_to_intent`, add status and event assertions:

```rust
assert_eq!(core.intents()[0].status, IntentStatus::Bound);
assert_eq!(core.events()[2].kind, EventKind::TaskCreated);
assert_eq!(core.events()[2].intent, Some(intent));
assert_eq!(core.events()[2].task, Some(task));
assert_eq!(core.events()[3].kind, EventKind::IntentBound);
assert_eq!(core.events()[3].intent, Some(intent));
assert_eq!(core.events()[3].task, Some(task));
assert_eq!(core.events()[3].verification, VerificationRequirement::Required);
```

- [ ] **Step 4: Add duplicate bind rejection test**

Append this test to `crates/agent-kernel-core/tests/intent_store.rs`:

```rust
#[test]
fn create_task_rejects_already_bound_intent_without_mutation() {
    let mut core = TestCore::new();
    let agent = AgentId::new(10);
    let resource = core
        .register_resource(ResourceKind::Workspace, None)
        .expect("resource should fit");
    let capability = core
        .grant_capability(agent, resource, OperationSet::only(Operation::Act))
        .expect("capability should fit");
    let intent = core
        .declare_intent(
            agent,
            capability,
            resource,
            IntentKind::Act,
            VerificationRequirement::Required,
        )
        .expect("intent should be declared");
    let first = core
        .create_task(agent, capability, intent)
        .expect("first task should be created");
    let events_after_first = core.events().len();

    let result = core.create_task(agent, capability, intent);

    assert_eq!(result, Err(KernelError::IntentStatusMismatch));
    assert_eq!(core.tasks().len(), 1);
    assert_eq!(core.tasks()[0].id, first);
    assert_eq!(core.intents()[0].status, IntentStatus::Bound);
    assert_eq!(core.events().len(), events_after_first);
}
```

- [ ] **Step 5: Add create-task event capacity atomicity test**

Append this test:

```rust
#[test]
fn create_task_requires_two_event_slots_without_mutation() {
    let mut core = KernelCore::<1, 1, 2, 1, 1, 0>::new();
    let agent = AgentId::new(11);
    let resource = core
        .register_resource(ResourceKind::Workspace, None)
        .expect("resource should fit");
    let capability = core
        .grant_capability(agent, resource, OperationSet::only(Operation::Act))
        .expect("grant should consume one event");
    let intent = core
        .declare_intent(
            agent,
            capability,
            resource,
            IntentKind::Act,
            VerificationRequirement::Required,
        )
        .expect("intent declaration should consume second event");

    let result = core.create_task(agent, capability, intent);

    assert_eq!(result, Err(KernelError::EventLogFull));
    assert!(core.tasks().is_empty());
    assert_eq!(core.intents()[0].status, IntentStatus::Declared);
    assert_eq!(core.events().len(), 2);
}
```

- [ ] **Step 6: Add verify fulfillment test**

Append this test:

```rust
#[test]
fn verify_task_fulfills_bound_intent_and_records_event() {
    let mut core = TestCore::new();
    let owner = AgentId::new(12);
    let assignee = AgentId::new(13);
    let owner_capability = grant_owner_capability(&mut core, owner);
    let resource = core.events()[0]
        .resource
        .expect("grant event should identify resource");
    let intent = core
        .declare_intent(
            owner,
            owner_capability,
            resource,
            IntentKind::Act,
            VerificationRequirement::Required,
        )
        .expect("intent should be declared");
    let task = core
        .create_task(owner, owner_capability, intent)
        .expect("task should be created");
    core.delegate_task(owner, owner_capability, task, assignee)
        .expect("task should delegate");
    let assignee_capability = core.tasks()[0]
        .delegated_capability
        .expect("delegation should derive capability");
    core.accept_task(assignee, task)
        .expect("task should be accepted");
    core.enqueue_task(assignee, task)
        .expect("task should enqueue");
    core.dispatch_next(assignee).expect("task should dispatch");
    core.complete_task(assignee, assignee_capability, task)
        .expect("task should complete");

    core.verify_task(owner, owner_capability, task)
        .expect("task should verify");

    assert_eq!(core.tasks()[0].status, TaskStatus::Verified);
    assert_eq!(core.intents()[0].status, IntentStatus::Fulfilled);
    let fulfilled = core.events().last().expect("fulfilled event should exist");
    assert_eq!(fulfilled.kind, EventKind::IntentFulfilled);
    assert_eq!(fulfilled.intent, Some(intent));
    assert_eq!(fulfilled.task, Some(task));
    assert_eq!(fulfilled.verification, VerificationRequirement::Required);
}
```

- [ ] **Step 7: Add cancellation test**

Append this test:

```rust
#[test]
fn cancel_task_cancels_bound_intent_and_records_event() {
    let mut core = TestCore::new();
    let owner = AgentId::new(14);
    let resource = core
        .register_resource(ResourceKind::Workspace, None)
        .expect("resource should fit");
    let capability = core
        .grant_capability(
            owner,
            resource,
            OperationSet::empty()
                .with(Operation::Act)
                .with(Operation::Rollback),
        )
        .expect("capability should fit");
    let intent = core
        .declare_intent(
            owner,
            capability,
            resource,
            IntentKind::Act,
            VerificationRequirement::Required,
        )
        .expect("intent should be declared");
    let task = core
        .create_task(owner, capability, intent)
        .expect("task should be created");

    core.cancel_task(owner, capability, task)
        .expect("task should be cancelled");

    assert_eq!(core.tasks()[0].status, TaskStatus::Cancelled);
    assert_eq!(core.intents()[0].status, IntentStatus::Cancelled);
    let cancelled = core.events().last().expect("cancel event should exist");
    assert_eq!(cancelled.kind, EventKind::IntentCancelled);
    assert_eq!(cancelled.intent, Some(intent));
    assert_eq!(cancelled.task, Some(task));
}
```

- [ ] **Step 8: Add verify event-capacity atomicity test**

Append this test:

```rust
#[test]
fn verify_task_requires_two_event_slots_without_mutation() {
    let mut core = KernelCore::<1, 4, 10, 1, 1, 1>::new();
    let owner = AgentId::new(15);
    let assignee = AgentId::new(16);
    let resource = core
        .register_resource(ResourceKind::Workspace, None)
        .expect("resource should fit");
    let owner_capability = core
        .grant_capability(
            owner,
            resource,
            OperationSet::empty()
                .with(Operation::Act)
                .with(Operation::Delegate)
                .with(Operation::Verify),
        )
        .expect("owner capability should fit");
    let intent = core
        .declare_intent(
            owner,
            owner_capability,
            resource,
            IntentKind::Act,
            VerificationRequirement::Required,
        )
        .expect("intent should be declared");
    let task = core
        .create_task(owner, owner_capability, intent)
        .expect("task should be created");
    core.delegate_task(owner, owner_capability, task, assignee)
        .expect("task should delegate");
    let assignee_capability = core.tasks()[0]
        .delegated_capability
        .expect("delegation should derive capability");
    core.accept_task(assignee, task)
        .expect("task should be accepted");
    core.enqueue_task(assignee, task)
        .expect("task should enqueue");
    core.dispatch_next(assignee).expect("task should dispatch");
    core.complete_task(assignee, assignee_capability, task)
        .expect("task should complete and consume final event slot");

    let result = core.verify_task(owner, owner_capability, task);

    assert_eq!(result, Err(KernelError::EventLogFull));
    assert_eq!(core.tasks()[0].status, TaskStatus::Completed);
    assert_eq!(core.intents()[0].status, IntentStatus::Bound);
    assert_eq!(core.events().len(), 10);
}
```

- [ ] **Step 9: Add cancel event-capacity atomicity test**

Append this test:

```rust
#[test]
fn cancel_task_requires_two_event_slots_without_mutation() {
    let mut core = KernelCore::<1, 1, 4, 1, 1, 0>::new();
    let owner = AgentId::new(17);
    let resource = core
        .register_resource(ResourceKind::Workspace, None)
        .expect("resource should fit");
    let capability = core
        .grant_capability(
            owner,
            resource,
            OperationSet::empty()
                .with(Operation::Act)
                .with(Operation::Rollback),
        )
        .expect("capability should fit");
    let intent = core
        .declare_intent(
            owner,
            capability,
            resource,
            IntentKind::Act,
            VerificationRequirement::Required,
        )
        .expect("intent should be declared");
    let task = core
        .create_task(owner, capability, intent)
        .expect("task should be created and bound");

    let result = core.cancel_task(owner, capability, task);

    assert_eq!(result, Err(KernelError::EventLogFull));
    assert_eq!(core.tasks()[0].status, TaskStatus::Created);
    assert_eq!(core.intents()[0].status, IntentStatus::Bound);
    assert_eq!(core.events().len(), 4);
}
```

- [ ] **Step 10: Run red core lifecycle tests**

Run:

```bash
PATH="$HOME/.cargo/bin:$PATH" RUSTC="$(rustup which rustc --toolchain nightly)" rustup run nightly cargo test -p agent-kernel-core --test intent_store
```

Expected: compile failures for missing `IntentStatus`, missing `Intent.status`, missing `EventKind::IntentBound`, missing `EventKind::IntentFulfilled`, missing `EventKind::IntentCancelled`, and missing `KernelError::IntentStatusMismatch`.

## Task 2: Core Intent Status Model And Events

**Files:**
- Modify: `crates/agent-kernel-core/src/intent.rs`
- Modify: `crates/agent-kernel-core/src/intent_store.rs`
- Create: `crates/agent-kernel-core/src/intent_event.rs`
- Modify: `crates/agent-kernel-core/src/error.rs`
- Modify: `crates/agent-kernel-core/src/event.rs`
- Modify: `crates/agent-kernel-core/src/lib.rs`

- [ ] **Step 1: Add `IntentStatus` and store it on `Intent`**

In `crates/agent-kernel-core/src/intent.rs`, add:

```rust
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
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
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Intent {
    pub id: IntentId,
    pub owner: AgentId,
    pub resource: ResourceId,
    pub kind: IntentKind,
    pub verification: VerificationRequirement,
    pub status: IntentStatus,
}
```

Update `Intent::empty()`:

```rust
pub(crate) const fn empty() -> Self {
    Self {
        id: IntentId::new(0),
        owner: AgentId::new(0),
        resource: ResourceId::new(0),
        kind: IntentKind::Act,
        verification: VerificationRequirement::Optional,
        status: IntentStatus::Declared,
    }
}
```

- [ ] **Step 2: Initialize declared intents with status**

In `crates/agent-kernel-core/src/intent_store.rs`, update the `Intent` literal in `declare_intent`:

```rust
self.intents[self.intent_len] = Intent {
    id: intent,
    owner: agent,
    resource,
    kind,
    verification,
    status: IntentStatus::Declared,
};
```

Add `IntentStatus` to the import list.

- [ ] **Step 3: Add mutable intent lookup and status helpers**

In `crates/agent-kernel-core/src/intent_store.rs`, add methods inside the existing `impl`:

```rust
pub(crate) fn find_intent_mut(&mut self, id: IntentId) -> Result<&mut Intent, KernelError> {
    self.intents[..self.intent_len]
        .iter_mut()
        .find(|intent| intent.id == id)
        .ok_or(KernelError::IntentNotFound)
}

pub(crate) fn ensure_intent_status(
    &self,
    id: IntentId,
    expected: IntentStatus,
) -> Result<Intent, KernelError> {
    let intent = self.find_intent(id)?;
    if intent.status == expected {
        Ok(intent)
    } else {
        Err(KernelError::IntentStatusMismatch)
    }
}

pub(crate) fn set_intent_status(
    &mut self,
    id: IntentId,
    status: IntentStatus,
) -> Result<(), KernelError> {
    self.find_intent_mut(id)?.status = status;
    Ok(())
}
```

- [ ] **Step 4: Add lifecycle errors and event kinds**

In `crates/agent-kernel-core/src/error.rs`, add:

```rust
IntentStatusMismatch,
```

In `crates/agent-kernel-core/src/event.rs`, add variants after `IntentDeclared`:

```rust
IntentBound,
IntentFulfilled,
IntentCancelled,
```

- [ ] **Step 5: Add intent task event helper**

Create `crates/agent-kernel-core/src/intent_event.rs`:

```rust
//! Intent lifecycle event recording helpers.
//!
//! This module belongs to `agent-kernel-core`. It records task-driven intent
//! state transitions while keeping task mutation and event construction
//! separate and deterministic.

use crate::{
    AgentId, Event, EventKind, KernelCore, KernelError, OperationSet, TaskId,
};

impl<
        const RESOURCES: usize,
        const CAPS: usize,
        const EVENTS: usize,
        const INTENTS: usize,
        const TASKS: usize,
        const RUN_QUEUE: usize,
    > KernelCore<RESOURCES, CAPS, EVENTS, INTENTS, TASKS, RUN_QUEUE>
{
    pub(crate) fn record_intent_task_event(
        &mut self,
        kind: EventKind,
        agent: AgentId,
        task: TaskId,
    ) -> Result<Event, KernelError> {
        let task_record = self.find_task(task)?;
        let intent_record = self.find_intent(task_record.intent)?;
        self.record(Event {
            sequence: self.next_sequence,
            agent,
            kind,
            resource: Some(intent_record.resource),
            capability: None,
            source_capability: None,
            intent: Some(intent_record.id),
            intent_kind: Some(intent_record.kind),
            action: None,
            operation: Some(intent_record.kind.required_operation()),
            operations: OperationSet::empty(),
            verification: intent_record.verification,
            checkpoint: None,
            task: Some(task),
            target_agent: None,
        })
    }
}
```

- [ ] **Step 6: Register and export new types**

In `crates/agent-kernel-core/src/lib.rs`, add the module:

```rust
mod intent_event;
```

Update the intent export:

```rust
pub use intent::{Intent, IntentKind, IntentStatus, VerificationRequirement};
```

- [ ] **Step 7: Run focused core red-green check**

Run:

```bash
PATH="$HOME/.cargo/bin:$PATH" RUSTC="$(rustup which rustc --toolchain nightly)" rustup run nightly cargo test -p agent-kernel-core --test intent_store
```

Expected: compile succeeds farther, with failing assertions or errors from task lifecycle not yet moving intent status or recording lifecycle events.

## Task 3: Task-Driven Intent Status Transitions

**Files:**
- Modify: `crates/agent-kernel-core/src/task_store.rs`
- Modify: `crates/agent-kernel-core/tests/*.rs`

- [ ] **Step 1: Update imports**

In `crates/agent-kernel-core/src/task_store.rs`, import `IntentStatus`:

```rust
use crate::{
    AgentId, CapabilityId, Event, EventKind, IntentId, IntentStatus, KernelCore, KernelError,
    Operation, OperationSet, Task, TaskId, TaskStatus,
};
```

- [ ] **Step 2: Require declared intent and two event slots in `create_task`**

In `create_task`, after the owner check and before authorization, add:

```rust
if intent_record.status != IntentStatus::Declared {
    return Err(KernelError::IntentStatusMismatch);
}
```

Change the event-capacity check from one slot to two:

```rust
self.ensure_event_slots(2)?;
```

After recording `TaskCreated`, update status and record `IntentBound`:

```rust
self.record_task_event(EventKind::TaskCreated, agent, Some(capability), task, None)?;
self.set_intent_status(intent, IntentStatus::Bound)?;
self.record_intent_task_event(EventKind::IntentBound, agent, task)?;
Ok(task)
```

- [ ] **Step 3: Fulfill intent in `verify_task`**

In `verify_task`, after existing task status validation and before mutation, add:

```rust
self.ensure_intent_status(current.intent, IntentStatus::Bound)?;
self.ensure_event_slots(2)?;
```

Replace the old one-slot capacity check and final event call with:

```rust
self.find_task_mut(task)?.status = TaskStatus::Verified;
self.record_task_event(EventKind::TaskVerified, agent, Some(capability), task, None)?;
self.set_intent_status(current.intent, IntentStatus::Fulfilled)?;
self.record_intent_task_event(EventKind::IntentFulfilled, agent, task)
```

- [ ] **Step 4: Cancel intent in `cancel_task`**

In `cancel_task`, after task status validation and before mutation, add:

```rust
self.ensure_intent_status(current.intent, IntentStatus::Bound)?;
self.ensure_event_slots(2)?;
```

Replace the old one-slot capacity check and final event call with:

```rust
self.find_task_mut(task)?.status = TaskStatus::Cancelled;
self.record_task_event(
    EventKind::TaskCancelled,
    agent,
    Some(capability),
    task,
    None,
)?;
self.set_intent_status(current.intent, IntentStatus::Cancelled)?;
self.record_intent_task_event(EventKind::IntentCancelled, agent, task)
```

- [ ] **Step 5: Update focused existing core assertions**

In `crates/agent-kernel-core/tests/intent_store.rs`, update `task_lifecycle_events_carry_task_intent` event range from:

```rust
for event in &core.events()[2..=9] {
    assert_eq!(event.intent, Some(intent));
}
```

to:

```rust
for event in &core.events()[2..=11] {
    assert_eq!(event.intent, Some(intent));
}
assert_eq!(core.events()[3].kind, EventKind::IntentBound);
assert_eq!(core.events()[11].kind, EventKind::IntentFulfilled);
```

- [ ] **Step 6: Run focused intent lifecycle test**

Run:

```bash
PATH="$HOME/.cargo/bin:$PATH" RUSTC="$(rustup which rustc --toolchain nightly)" rustup run nightly cargo test -p agent-kernel-core --test intent_store
```

Expected: all tests in `intent_store.rs` pass.

- [ ] **Step 7: Run all core tests and inspect failures**

Run:

```bash
PATH="$HOME/.cargo/bin:$PATH" RUSTC="$(rustup which rustc --toolchain nightly)" rustup run nightly cargo test -p agent-kernel-core
```

Expected: failures in existing tests that assert fixed event indexes or small event capacities.

- [ ] **Step 8: Update existing core tests for new event ordering**

Use:

```bash
rg -n "IntentDeclared|TaskCreated|CapabilityDerived|DelegationRequested|TaskVerified|TaskCancelled|events\\(\\)\\[[0-9]+\\]|KernelCore::<" crates/agent-kernel-core/tests -g '*.rs'
```

Update expected sequences:

```text
CapabilityGranted
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

For cancellation tests with a created task:

```text
CapabilityGranted
IntentDeclared
TaskCreated
IntentBound
TaskCancelled
IntentCancelled
```

Increase event capacities in test type aliases only where tests now need the two additional lifecycle events. Keep `INTENTS`, `TASKS`, and `RUN_QUEUE` capacities unchanged unless the test creates more records.

Apply these concrete updates while fixing the core test suite:

If a test file asserts `IntentStatus`, add it to that file's
`agent_kernel_core` import list:

```rust
IntentStatus,
```

```rust
// crates/agent-kernel-core/tests/task_lifecycle.rs
assert_eq!(core.events()[3].kind, EventKind::IntentBound);
assert_eq!(core.events()[4].kind, EventKind::CapabilityDerived);
assert_eq!(core.events()[5].kind, EventKind::DelegationRequested);
assert_eq!(core.events()[6].kind, EventKind::TaskAccepted);
assert_eq!(core.events()[7].kind, EventKind::TaskQueued);
assert_eq!(core.events()[8].kind, EventKind::TaskDispatched);
assert_eq!(core.events()[9].kind, EventKind::TaskCompleted);
assert_eq!(core.events()[10].kind, EventKind::TaskVerified);
assert_eq!(core.events()[11].kind, EventKind::IntentFulfilled);
for event in &core.events()[2..=11] {
    assert_eq!(event.intent, Some(intent));
}
```

```rust
// crates/agent-kernel-core/tests/capability_lifecycle.rs
assert_eq!(events[4].kind, EventKind::CapabilityDerived);
assert_eq!(events[4].intent, Some(intent));
assert_eq!(events[5].kind, EventKind::DelegationRequested);
```

```rust
// crates/agent-kernel-core/tests/task_authority.rs
let second_intent = core
    .declare_intent(
        agent,
        capability,
        resource,
        IntentKind::Act,
        VerificationRequirement::Required,
    )
    .expect("second intent should be declared");
let result = core.create_task(agent, capability, second_intent);
assert_eq!(result, Err(KernelError::TaskStoreFull));
```

When a test checks cancellation, assert the final lifecycle event explicitly:

```rust
assert_eq!(core.events().last().unwrap().kind, EventKind::IntentCancelled);
assert_eq!(core.intents()[0].status, IntentStatus::Cancelled);
```

- [ ] **Step 9: Run core tests again**

Run:

```bash
PATH="$HOME/.cargo/bin:$PATH" RUSTC="$(rustup which rustc --toolchain nightly)" rustup run nightly cargo test -p agent-kernel-core
```

Expected: all core tests pass.

## Task 4: Facade, Supervisor, Boot Match, And README

**Files:**
- Modify: `crates/agent-kernel/tests/intent_lifecycle.rs`
- Modify: `crates/agent-kernel/tests/kernel_facade.rs`
- Modify: `crates/agent-supervisor/src/main.rs`
- Modify: `crates/agent-supervisor/tests/supervisor_flow.rs`
- Modify: `crates/agent-kernel-x86_64/src/main.rs`
- Modify: `README.md`

- [ ] **Step 1: Update facade lifecycle tests**

In `crates/agent-kernel/tests/intent_lifecycle.rs`, add `IntentStatus` to imports:

```rust
use agent_kernel_core::{
    AgentId, EventKind, IntentId, IntentKind, IntentStatus, Operation, OperationSet,
    ResourceKind, TaskId, VerificationRequirement,
};
```

In `sys_declare_intent_records_and_exposes_intent`, add:

```rust
assert_eq!(kernel.intents()[0].status, IntentStatus::Declared);
```

In `sys_create_task_accepts_intent_id`, add:

```rust
assert_eq!(kernel.intents()[0].status, IntentStatus::Bound);
assert_eq!(kernel.events()[3].kind, EventKind::IntentBound);
assert_eq!(kernel.events()[3].intent, Some(intent));
assert_eq!(kernel.events()[3].task, Some(task));
```

- [ ] **Step 2: Add facade verify/cancel visibility tests**

Append these tests to `crates/agent-kernel/tests/intent_lifecycle.rs`:

```rust
#[test]
fn sys_verify_task_exposes_fulfilled_intent_status() {
    let mut kernel = TestKernel::new();
    let owner = AgentId::new(3);
    let assignee = AgentId::new(4);
    let resource = kernel
        .sys_register_resource(ResourceKind::Workspace, None)
        .expect("resource should fit");
    let capability = kernel
        .sys_grant(
            owner,
            resource,
            OperationSet::empty()
                .with(Operation::Act)
                .with(Operation::Delegate)
                .with(Operation::Verify),
        )
        .expect("capability should fit");
    let intent = kernel
        .sys_declare_intent(
            owner,
            capability,
            resource,
            IntentKind::Act,
            VerificationRequirement::Required,
        )
        .expect("intent should be declared");
    let task = kernel
        .sys_create_task(owner, capability, intent)
        .expect("task should be created");
    kernel
        .sys_delegate_task(owner, capability, task, assignee)
        .expect("task should delegate");
    let assignee_capability = kernel.tasks()[0]
        .delegated_capability
        .expect("delegation should derive capability");
    kernel
        .sys_accept_task(assignee, task)
        .expect("task should accept");
    kernel
        .sys_enqueue_task(assignee, task)
        .expect("task should enqueue");
    kernel
        .sys_dispatch_next(assignee)
        .expect("task should dispatch");
    kernel
        .sys_complete_task(assignee, assignee_capability, task)
        .expect("task should complete");

    kernel
        .sys_verify_task(owner, capability, task)
        .expect("task should verify");

    assert_eq!(kernel.intents()[0].status, IntentStatus::Fulfilled);
    assert_eq!(
        kernel.events().last().expect("intent event should exist").kind,
        EventKind::IntentFulfilled
    );
}

#[test]
fn sys_cancel_task_exposes_cancelled_intent_status() {
    let mut kernel = TestKernel::new();
    let owner = AgentId::new(5);
    let resource = kernel
        .sys_register_resource(ResourceKind::Workspace, None)
        .expect("resource should fit");
    let capability = kernel
        .sys_grant(
            owner,
            resource,
            OperationSet::empty()
                .with(Operation::Act)
                .with(Operation::Rollback),
        )
        .expect("capability should fit");
    let intent = kernel
        .sys_declare_intent(
            owner,
            capability,
            resource,
            IntentKind::Act,
            VerificationRequirement::Required,
        )
        .expect("intent should be declared");
    let task = kernel
        .sys_create_task(owner, capability, intent)
        .expect("task should be created");

    kernel
        .sys_cancel_task(owner, capability, task)
        .expect("task should cancel");

    assert_eq!(kernel.intents()[0].status, IntentStatus::Cancelled);
    assert_eq!(
        kernel.events().last().expect("intent event should exist").kind,
        EventKind::IntentCancelled
    );
}
```

- [ ] **Step 3: Update facade full lifecycle event sequence**

In `crates/agent-kernel/tests/kernel_facade.rs`, update the full lifecycle assertion sequence to:

```rust
assert_eq!(kernel.events()[0].kind, EventKind::CapabilityGranted);
assert_eq!(kernel.events()[1].kind, EventKind::IntentDeclared);
assert_eq!(kernel.events()[2].kind, EventKind::TaskCreated);
assert_eq!(kernel.events()[3].kind, EventKind::IntentBound);
assert_eq!(kernel.events()[4].kind, EventKind::CapabilityDerived);
assert_eq!(kernel.events()[5].kind, EventKind::DelegationRequested);
assert_eq!(kernel.events()[6].kind, EventKind::TaskAccepted);
assert_eq!(kernel.events()[7].kind, EventKind::TaskQueued);
assert_eq!(kernel.events()[8].kind, EventKind::TaskDispatched);
assert_eq!(kernel.events()[9].kind, EventKind::TaskCompleted);
assert_eq!(kernel.events()[10].kind, EventKind::TaskVerified);
assert_eq!(kernel.events()[11].kind, EventKind::IntentFulfilled);
for event in &kernel.events()[2..=11] {
    assert_eq!(event.intent, Some(intent));
}
```

Run:

```bash
PATH="$HOME/.cargo/bin:$PATH" RUSTC="$(rustup which rustc --toolchain nightly)" rustup run nightly cargo test -p agent-kernel
```

Expected: all facade tests pass. If `crates/agent-kernel/tests/intent_lifecycle.rs`
needs more event capacity after adding the verify and cancel tests, change its
alias from `AgentKernel<4, 4, 16, 4, 4, 4>` to
`AgentKernel<4, 6, 32, 4, 4, 4>`.

- [ ] **Step 4: Update supervisor event formatting**

In `crates/agent-supervisor/src/main.rs`, extend the `match event.kind` block:

```rust
EventKind::IntentDeclared => format_intent_event(event, "intent_declared"),
EventKind::IntentBound => format_intent_event(event, "intent_bound"),
EventKind::IntentFulfilled => format_intent_event(event, "intent_fulfilled"),
EventKind::IntentCancelled => format_intent_event(event, "intent_cancelled"),
```

Change `format_intent_event` signature and label usage:

```rust
fn format_intent_event(event: &Event, label: &str) -> String {
    let agent = event.agent.raw();
    let resource = event
        .resource
        .map(|resource| resource.raw())
        .unwrap_or_default();
    let intent = event.intent.map(|intent| intent.raw()).unwrap_or_default();

    format!(
        "event[{}] {} agent={} resource={} intent={}",
        event.sequence, label, agent, resource, intent
    )
}
```

- [ ] **Step 5: Update supervisor expected output**

In `crates/agent-supervisor/tests/supervisor_flow.rs`, update event assertions:

```rust
assert!(stdout.contains("event[7] intent_declared agent=1 resource=1 intent=1"));
assert!(stdout.contains("event[8] task_created agent=1 resource=1 task=1"));
assert!(stdout.contains("event[9] intent_bound agent=1 resource=1 intent=1"));
assert!(stdout.contains("event[10] capability_derived agent=1 resource=1 capability=2"));
assert!(stdout.contains("event[11] delegation agent=1 resource=1 task=1 target_agent=2"));
assert!(stdout.contains("event[12] task_accepted agent=2 resource=1 task=1"));
assert!(stdout.contains("event[13] task_queued agent=2 resource=1 task=1"));
assert!(stdout.contains("event[14] task_dispatched agent=2 resource=1 task=1"));
assert!(stdout.contains("event[15] task_completed agent=2 resource=1 task=1"));
assert!(stdout.contains("event[16] task_verified agent=1 resource=1 task=1"));
assert!(stdout.contains("event[17] intent_fulfilled agent=1 resource=1 intent=1"));
```

- [ ] **Step 6: Update x86 exhaustive match**

In `crates/agent-kernel-x86_64/src/main.rs`, add match arms:

```rust
EventKind::IntentBound => {
    serial_write_line("intent_bound");
}
EventKind::IntentFulfilled => {
    serial_write_line("intent_fulfilled");
}
EventKind::IntentCancelled => {
    serial_write_line("intent_cancelled");
}
```

QEMU boot still prints only capability/observation/action/verification because boot does not declare intents.

- [ ] **Step 7: Update README current behavior and output**

In `README.md`, insert intent lifecycle steps into the current behavior list:

```text
9. Declare a typed action intent that requires verification.
10. Create a kernel-owned task from that intent.
11. Bind the intent to the task.
12. Delegate the task to another agent.
13. Record the derived task-scoped capability in the kernel event log.
14. Let the assignee accept the task.
15. Enqueue the accepted task and dispatch it into `Running` state through the kernel run queue.
16. Let the assignee complete the running task.
17. Request verification for the completed task.
18. Mark the intent fulfilled after task verification.
19. Print the kernel event log from the supervisor.
```

Update expected supervisor output:

```text
event[7] intent_declared agent=1 resource=1 intent=1
event[8] task_created agent=1 resource=1 task=1
event[9] intent_bound agent=1 resource=1 intent=1
event[10] capability_derived agent=1 resource=1 capability=2
event[11] delegation agent=1 resource=1 task=1 target_agent=2
event[12] task_accepted agent=2 resource=1 task=1
event[13] task_queued agent=2 resource=1 task=1
event[14] task_dispatched agent=2 resource=1 task=1
event[15] task_completed agent=2 resource=1 task=1
event[16] task_verified agent=1 resource=1 task=1
event[17] intent_fulfilled agent=1 resource=1 intent=1
```

## Task 5: Full Verification And Publish

**Files:**
- All files changed by Tasks 1-4.

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

Expected output includes:

```text
event[9] intent_bound agent=1 resource=1 intent=1
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

- [ ] **Step 5: Check file sizes**

Run:

```bash
wc -l crates/agent-kernel-core/src/*.rs crates/agent-kernel/src/*.rs crates/agent-supervisor/src/*.rs crates/agent-kernel-core/tests/*.rs crates/agent-kernel/tests/*.rs crates/agent-supervisor/tests/*.rs
```

Expected: no source file exceeds its hard limit. If a core module exceeds 220 lines, split by cohesive responsibility before committing. Existing tests may remain above the soft limit if they stay below the 500-line hard limit and the final report calls this out as test debt.

- [ ] **Step 6: Commit and push implementation**

Run:

```bash
git status --short
git add README.md crates docs
git commit -m "feat: add intent lifecycle"
git push
```

Expected: `origin/main` receives the implementation commit.

## Self-Review

Spec coverage:

- `IntentStatus` data model is implemented by Tasks 1-2.
- Task-driven `Declared -> Bound -> Fulfilled/Cancelled` transitions are implemented by Task 3.
- Event kinds and supervisor output are implemented by Tasks 2 and 4.
- Atomic event-capacity behavior is tested in Task 1 and implemented in Task 3.
- no_std boundary preservation is verified by workspace tests and QEMU in Task 5.

Open item scan:

- This plan contains concrete file paths, code snippets, commands, and expected results.
- It does not contain open-ended implementation gaps.

Type consistency:

- `IntentStatus`, `IntentBound`, `IntentFulfilled`, `IntentCancelled`, and `IntentStatusMismatch` match the design spec.
- Public facade behavior continues through existing task syscalls and read-only `intents()`.
