# Checkpoint Store V0 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add deterministic, queryable kernel records for checkpoint creation and rollback requests.

**Architecture:** `agent-kernel-core` gains a fixed-capacity checkpoint store, and checkpoint/rollback requests mutate that store atomically with event recording. Rollback remains a request state in V0; it does not claim that any resource snapshot has been restored.

**Tech Stack:** Rust nightly, no_std core and facade crates, fixed-capacity arrays, Cargo workspace tests, QEMU BIOS boot script.

---

## File Structure

- Create `crates/agent-kernel-core/src/checkpoint.rs` for `CheckpointRecord` and `CheckpointStatus`.
- Create `crates/agent-kernel-core/src/checkpoint_store.rs` for checkpoint lookup, creation, rollback-request transitions, and read-only inspection.
- Modify `crates/agent-kernel-core/src/core.rs` for checkpoint capacity and base state initialization; remove old event-only `checkpoint` and `rollback`.
- Modify every `crates/agent-kernel-core/src/*.rs` file with a `KernelCore<...>` impl header to include `CHECKPOINTS`.
- Modify `crates/agent-kernel-core/src/error.rs` and `lib.rs` for public errors and exports.
- Create `crates/agent-kernel-core/tests/checkpoint_store.rs` and `crates/agent-kernel-core/tests/checkpoint_rollback.rs` for red/green core store behavior.
- Modify existing core tests that use old `KernelCore` type arity or only assert checkpoint events.
- Modify `crates/agent-kernel/src/lib.rs` and `scheduler.rs` for new `AgentKernel` type arity and read-only `checkpoints()` access.
- Modify `crates/agent-kernel/tests/kernel_facade.rs`, `intent_lifecycle.rs`, and `capability_lifecycle.rs` for facade visibility and generic arity.
- Modify `crates/agent-kernel-boot/src/lib.rs`, `crates/agent-kernel-boot/tests/boot_flow.rs`, and `crates/agent-kernel-x86_64/src/main.rs` for capacity arity.
- Modify `crates/agent-supervisor/src/main.rs` only for type arity; expected output remains unchanged.
- Modify `README.md` to mention checkpoint records in current behavior.

## Task 1: Core Checkpoint Red Tests

**Files:**
- Create: `crates/agent-kernel-core/tests/checkpoint_store.rs`
- Create: `crates/agent-kernel-core/tests/checkpoint_rollback.rs`

- [x] **Step 1: Create focused red tests**

Create `crates/agent-kernel-core/tests/checkpoint_store.rs`:

```rust
use agent_kernel_core::{
    AgentId, CheckpointId, CheckpointStatus, EventKind, KernelCore, KernelError, Operation,
    OperationSet, ResourceKind,
};

type TestCore = KernelCore<4, 4, 16, 2, 2, 4, 0, 0, 0>;

#[test]
fn checkpoint_records_checkpoint_and_event() {
    let mut core = TestCore::new();
    let agent = AgentId::new(1);
    let resource = core
        .register_resource(ResourceKind::Workspace, None)
        .expect("resource should fit");
    let capability = core
        .grant_capability(agent, resource, OperationSet::only(Operation::Checkpoint))
        .expect("capability should fit");
    let checkpoint = CheckpointId::new(7);
    let events_after_grant = core.events().len();

    let event = core
        .checkpoint(agent, capability, checkpoint, resource)
        .expect("checkpoint should record");

    assert_eq!(core.checkpoints().len(), 1);
    assert_eq!(core.checkpoints()[0].id, checkpoint);
    assert_eq!(core.checkpoints()[0].agent, agent);
    assert_eq!(core.checkpoints()[0].resource, resource);
    assert_eq!(core.checkpoints()[0].capability, capability);
    assert_eq!(core.checkpoints()[0].status, CheckpointStatus::Created);
    assert_eq!(event.kind, EventKind::CheckpointCreated);
    assert_eq!(event.checkpoint, Some(checkpoint));
    assert_eq!(event.operation, Some(Operation::Checkpoint));
    assert_eq!(core.events().len(), events_after_grant + 1);
    assert_eq!(
        core.events()[events_after_grant].kind,
        EventKind::CheckpointCreated
    );
    assert_eq!(core.events()[events_after_grant].checkpoint, Some(checkpoint));
}

#[test]
fn checkpoint_rejects_duplicate_without_event() {
    let mut core = TestCore::new();
    let agent = AgentId::new(2);
    let resource = core
        .register_resource(ResourceKind::Workspace, None)
        .expect("resource should fit");
    let capability = core
        .grant_capability(agent, resource, OperationSet::only(Operation::Checkpoint))
        .expect("capability should fit");
    let checkpoint = CheckpointId::new(8);
    core.checkpoint(agent, capability, checkpoint, resource)
        .expect("first checkpoint should record");
    let events_after_checkpoint = core.events().len();

    let result = core.checkpoint(agent, capability, checkpoint, resource);

    assert_eq!(result, Err(KernelError::CheckpointAlreadyExists));
    assert_eq!(core.checkpoints().len(), 1);
    assert_eq!(core.checkpoints()[0].status, CheckpointStatus::Created);
    assert_eq!(core.events().len(), events_after_checkpoint);
}

#[test]
fn checkpoint_store_full_leaves_events_unchanged() {
    let mut core = KernelCore::<1, 1, 4, 1, 1, 0, 0, 0, 0>::new();
    let agent = AgentId::new(3);
    let resource = core
        .register_resource(ResourceKind::Workspace, None)
        .expect("resource should fit");
    let capability = core
        .grant_capability(agent, resource, OperationSet::only(Operation::Checkpoint))
        .expect("capability should fit");
    let events_after_grant = core.events().len();
    let grant_event = core.events()[events_after_grant - 1];

    let result = core.checkpoint(agent, capability, CheckpointId::new(9), resource);

    assert_eq!(result, Err(KernelError::CheckpointStoreFull));
    assert!(core.checkpoints().is_empty());
    assert_eq!(core.events().len(), events_after_grant);
    assert_eq!(core.events()[events_after_grant - 1], grant_event);
}

#[test]
fn checkpoint_event_log_full_leaves_checkpoints_unchanged() {
    let mut core = KernelCore::<1, 1, 1, 1, 1, 1, 0, 0, 0>::new();
    let agent = AgentId::new(4);
    let resource = core
        .register_resource(ResourceKind::Workspace, None)
        .expect("resource should fit");
    let capability = core
        .grant_capability(agent, resource, OperationSet::only(Operation::Checkpoint))
        .expect("grant should consume only event slot");
    let grant_event = core.events()[0];

    let result = core.checkpoint(agent, capability, CheckpointId::new(10), resource);

    assert_eq!(result, Err(KernelError::EventLogFull));
    assert!(core.checkpoints().is_empty());
    assert_eq!(core.events().len(), 1);
    assert_eq!(core.events()[0], grant_event);
}

#[test]
fn rollback_existing_checkpoint_updates_status_and_event() {
    let mut core = TestCore::new();
    let agent = AgentId::new(5);
    let resource = core
        .register_resource(ResourceKind::Workspace, None)
        .expect("resource should fit");
    let capability = core
        .grant_capability(
            agent,
            resource,
            OperationSet::empty()
                .with(Operation::Checkpoint)
                .with(Operation::Rollback),
        )
        .expect("capability should fit");
    let checkpoint = CheckpointId::new(11);
    core.checkpoint(agent, capability, checkpoint, resource)
        .expect("checkpoint should record");
    let events_after_checkpoint = core.events().len();

    let event = core
        .rollback(agent, capability, checkpoint, resource)
        .expect("rollback should record");

    assert_eq!(
        core.checkpoints()[0].status,
        CheckpointStatus::RollbackRequested
    );
    assert_eq!(event.kind, EventKind::RollbackRequested);
    assert_eq!(event.checkpoint, Some(checkpoint));
    assert_eq!(event.operation, Some(Operation::Rollback));
    assert_eq!(core.events().len(), events_after_checkpoint + 1);
    assert_eq!(
        core.events()[events_after_checkpoint].kind,
        EventKind::RollbackRequested
    );
}

#[test]
fn rollback_missing_checkpoint_leaves_events_unchanged() {
    let mut core = TestCore::new();
    let agent = AgentId::new(6);
    let resource = core
        .register_resource(ResourceKind::Workspace, None)
        .expect("resource should fit");
    let capability = core
        .grant_capability(agent, resource, OperationSet::only(Operation::Rollback))
        .expect("capability should fit");
    let events_after_grant = core.events().len();
    let grant_event = core.events()[events_after_grant - 1];

    let result = core.rollback(agent, capability, CheckpointId::new(12), resource);

    assert_eq!(result, Err(KernelError::CheckpointNotFound));
    assert!(core.checkpoints().is_empty());
    assert_eq!(core.events().len(), events_after_grant);
    assert_eq!(core.events()[events_after_grant - 1], grant_event);
}

#[test]
fn rollback_rejects_checkpoint_resource_mismatch_without_status_change() {
    let mut core = KernelCore::<2, 2, 8, 1, 1, 2, 0, 0, 0>::new();
    let agent = AgentId::new(7);
    let first_resource = core
        .register_resource(ResourceKind::Workspace, None)
        .expect("first resource should fit");
    let second_resource = core
        .register_resource(ResourceKind::Service, None)
        .expect("second resource should fit");
    let checkpoint_capability = core
        .grant_capability(agent, first_resource, OperationSet::only(Operation::Checkpoint))
        .expect("checkpoint capability should fit");
    let rollback_capability = core
        .grant_capability(agent, second_resource, OperationSet::only(Operation::Rollback))
        .expect("rollback capability should fit");
    let checkpoint = CheckpointId::new(13);
    core.checkpoint(agent, checkpoint_capability, checkpoint, first_resource)
        .expect("checkpoint should record");
    let events_after_checkpoint = core.events().len();
    let checkpoint_status_before = core.checkpoints()[0].status;

    let result = core.rollback(agent, rollback_capability, checkpoint, second_resource);

    assert_eq!(result, Err(KernelError::CheckpointResourceMismatch));
    assert_eq!(core.checkpoints().len(), 1);
    assert_eq!(core.checkpoints()[0].status, checkpoint_status_before);
    assert_eq!(core.events().len(), events_after_checkpoint);
}

#[test]
fn rollback_rejects_repeated_request_without_event() {
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
                .with(Operation::Checkpoint)
                .with(Operation::Rollback),
        )
        .expect("capability should fit");
    let checkpoint = CheckpointId::new(14);
    core.checkpoint(agent, capability, checkpoint, resource)
        .expect("checkpoint should record");
    core.rollback(agent, capability, checkpoint, resource)
        .expect("first rollback should record");
    let events_after_rollback = core.events().len();

    let result = core.rollback(agent, capability, checkpoint, resource);

    assert_eq!(result, Err(KernelError::CheckpointStatusMismatch));
    assert_eq!(core.checkpoints().len(), 1);
    assert_eq!(
        core.checkpoints()[0].status,
        CheckpointStatus::RollbackRequested
    );
    assert_eq!(core.events().len(), events_after_rollback);
}

#[test]
fn rollback_event_log_full_leaves_checkpoint_status_created() {
    let mut core = KernelCore::<1, 1, 2, 1, 1, 1, 0, 0, 0>::new();
    let agent = AgentId::new(9);
    let resource = core
        .register_resource(ResourceKind::Workspace, None)
        .expect("resource should fit");
    let capability = core
        .grant_capability(
            agent,
            resource,
            OperationSet::empty()
                .with(Operation::Checkpoint)
                .with(Operation::Rollback),
        )
        .expect("grant should consume first event");
    let checkpoint = CheckpointId::new(15);
    core.checkpoint(agent, capability, checkpoint, resource)
        .expect("checkpoint should consume second event");
    let events_after_checkpoint = core.events().len();

    let result = core.rollback(agent, capability, checkpoint, resource);

    assert_eq!(result, Err(KernelError::EventLogFull));
    assert_eq!(core.checkpoints().len(), 1);
    assert_eq!(core.checkpoints()[0].status, CheckpointStatus::Created);
    assert_eq!(core.events().len(), events_after_checkpoint);
}
```

- [x] **Step 2: Run the red test**

Run:

```bash
RUSTC="$(rustup which rustc --toolchain nightly)" RUSTDOC="$(rustup which rustdoc --toolchain nightly)" rustup run nightly cargo test -p agent-kernel-core --test checkpoint_store
```

Expected: compile failures for missing `CheckpointStatus`, `checkpoints`, new
`KernelError` variants, and new `KernelCore` generic arity.

## Task 2: Core Checkpoint Data Model

**Files:**
- Create: `crates/agent-kernel-core/src/checkpoint.rs`
- Modify: `crates/agent-kernel-core/src/error.rs`
- Modify: `crates/agent-kernel-core/src/core.rs`
- Modify: `crates/agent-kernel-core/src/lib.rs`

- [x] **Step 1: Add checkpoint record types**

Create `crates/agent-kernel-core/src/checkpoint.rs`:

```rust
//! Kernel-owned checkpoint records.
//!
//! This module belongs to `agent-kernel-core`. It defines copyable checkpoint
//! records for the fixed-capacity no_std checkpoint store. It does not snapshot
//! or restore resource state.

use crate::{AgentId, CapabilityId, CheckpointId, ResourceId};

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

impl CheckpointRecord {
    pub(crate) const fn empty() -> Self {
        Self {
            id: CheckpointId::new(0),
            agent: AgentId::new(0),
            resource: ResourceId::new(0),
            capability: CapabilityId::new(0),
            status: CheckpointStatus::Created,
        }
    }
}
```

- [x] **Step 2: Add errors**

In `crates/agent-kernel-core/src/error.rs`, add:

```rust
CheckpointStoreFull,
CheckpointAlreadyExists,
CheckpointNotFound,
CheckpointResourceMismatch,
CheckpointStatusMismatch,
```

- [x] **Step 3: Extend core state**

In `crates/agent-kernel-core/src/core.rs`, import `CheckpointRecord`, add
`const CHECKPOINTS: usize` after `OBSERVATIONS`, add:

```rust
pub(crate) checkpoints: [CheckpointRecord; CHECKPOINTS],
pub(crate) checkpoint_len: usize,
```

Initialize:

```rust
checkpoints: [CheckpointRecord::empty(); CHECKPOINTS],
checkpoint_len: 0,
```

Use the new generic order:

```rust
KernelCore<RESOURCES, CAPS, EVENTS, ACTIONS, OBSERVATIONS, CHECKPOINTS, INTENTS, TASKS, RUN_QUEUE>
```

- [x] **Step 4: Register exports**

In `crates/agent-kernel-core/src/lib.rs`, add:

```rust
mod checkpoint;
pub use checkpoint::{CheckpointRecord, CheckpointStatus};
```

- [x] **Step 5: Run focused check**

Run:

```bash
RUSTC="$(rustup which rustc --toolchain nightly)" RUSTDOC="$(rustup which rustdoc --toolchain nightly)" rustup run nightly cargo test -p agent-kernel-core --test checkpoint_store
```

Expected: compile failures now progress to unpropagated generic arity and
missing `checkpoints()` behavior.

## Task 3: Propagate Checkpoint Capacity Arity

**Files:**
- Modify: `crates/agent-kernel-core/src/*.rs`
- Modify: `crates/agent-kernel-core/tests/*.rs`

- [x] **Step 1: Update core impl headers**

For every `KernelCore<...>` impl in core source, add:

```rust
const CHECKPOINTS: usize,
```

after `const OBSERVATIONS: usize`, and use:

```rust
KernelCore<RESOURCES, CAPS, EVENTS, ACTIONS, OBSERVATIONS, CHECKPOINTS, INTENTS, TASKS, RUN_QUEUE>
```

- [x] **Step 2: Update core test aliases**

Update existing core test aliases by inserting checkpoint capacity after
observation capacity. Use small nonzero capacities unless the test intentionally
exhausts another store:

```rust
KernelCore<4, 4, 16, 4, 4, 4, 0, 4, 4>
KernelCore<4, 8, 32, 2, 2, 2, 6, 6, 4>
KernelCore<4, 8, 64, 2, 2, 2, 4, 6, 4>
KernelCore<4, 6, 32, 4, 2, 2, 6, 6, 4>
```

For one-off capacity tests, preserve the intended exhausted capacity and insert
a harmless checkpoint capacity before `INTENTS`.

- [x] **Step 3: Run core tests**

Run:

```bash
RUSTC="$(rustup which rustc --toolchain nightly)" RUSTDOC="$(rustup which rustdoc --toolchain nightly)" rustup run nightly cargo test -p agent-kernel-core
```

Expected: existing tests may still fail on checkpoint store behavior until Task
4, but generic arity errors should be gone.

## Task 4: Implement Checkpoint Store Behavior

**Files:**
- Create: `crates/agent-kernel-core/src/checkpoint_store.rs`
- Modify: `crates/agent-kernel-core/src/core.rs`
- Modify: `crates/agent-kernel-core/src/lib.rs`

- [x] **Step 1: Add checkpoint store module**

Create `crates/agent-kernel-core/src/checkpoint_store.rs`:

```rust
//! Fixed-capacity kernel checkpoint store behavior.
//!
//! This module records authorized checkpoints, tracks rollback requests, and
//! emits replayable events without allocation or resource snapshot execution.

use crate::{
    AgentId, CapabilityId, CheckpointId, CheckpointRecord, CheckpointStatus, Event, EventKind,
    KernelCore, KernelError, Operation, OperationSet, ResourceId, VerificationRequirement,
};

impl<
        const RESOURCES: usize,
        const CAPS: usize,
        const EVENTS: usize,
        const ACTIONS: usize,
        const OBSERVATIONS: usize,
        const CHECKPOINTS: usize,
        const INTENTS: usize,
        const TASKS: usize,
        const RUN_QUEUE: usize,
    >
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
{
    pub fn checkpoint(
        &mut self,
        agent: AgentId,
        capability: CapabilityId,
        checkpoint: CheckpointId,
        resource: ResourceId,
    ) -> Result<Event, KernelError> {
        self.ensure_authorized(agent, capability, resource, Operation::Checkpoint)?;
        if self.find_checkpoint(checkpoint).is_ok() {
            return Err(KernelError::CheckpointAlreadyExists);
        }
        if self.checkpoint_len >= CHECKPOINTS {
            return Err(KernelError::CheckpointStoreFull);
        }
        self.ensure_event_slots(1)?;

        self.checkpoints[self.checkpoint_len] = CheckpointRecord {
            id: checkpoint,
            agent,
            resource,
            capability,
            status: CheckpointStatus::Created,
        };
        self.checkpoint_len += 1;

        self.record(Event {
            sequence: 0,
            agent,
            kind: EventKind::CheckpointCreated,
            resource: Some(resource),
            capability: Some(capability),
            source_capability: None,
            intent: None,
            intent_kind: None,
            action: None,
            observation: None,
            operation: Some(Operation::Checkpoint),
            operations: OperationSet::empty(),
            verification: VerificationRequirement::Optional,
            checkpoint: Some(checkpoint),
            task: None,
            target_agent: None,
        })
    }

    pub fn rollback(
        &mut self,
        agent: AgentId,
        capability: CapabilityId,
        checkpoint: CheckpointId,
        resource: ResourceId,
    ) -> Result<Event, KernelError> {
        self.ensure_authorized(agent, capability, resource, Operation::Rollback)?;

        let record = self.find_checkpoint(checkpoint)?;
        if record.resource != resource {
            return Err(KernelError::CheckpointResourceMismatch);
        }
        if record.status != CheckpointStatus::Created {
            return Err(KernelError::CheckpointStatusMismatch);
        }
        self.ensure_event_slots(1)?;

        self.find_checkpoint_mut(checkpoint)?.status = CheckpointStatus::RollbackRequested;

        self.record(Event {
            sequence: 0,
            agent,
            kind: EventKind::RollbackRequested,
            resource: Some(resource),
            capability: Some(capability),
            source_capability: None,
            intent: None,
            intent_kind: None,
            action: None,
            observation: None,
            operation: Some(Operation::Rollback),
            operations: OperationSet::empty(),
            verification: VerificationRequirement::Optional,
            checkpoint: Some(checkpoint),
            task: None,
            target_agent: None,
        })
    }

    pub fn checkpoints(&self) -> &[CheckpointRecord] {
        &self.checkpoints[..self.checkpoint_len]
    }

    pub(crate) fn find_checkpoint(
        &self,
        id: CheckpointId,
    ) -> Result<CheckpointRecord, KernelError> {
        for checkpoint in self.checkpoints() {
            if checkpoint.id == id {
                return Ok(*checkpoint);
            }
        }

        Err(KernelError::CheckpointNotFound)
    }

    fn find_checkpoint_mut(
        &mut self,
        id: CheckpointId,
    ) -> Result<&mut CheckpointRecord, KernelError> {
        for checkpoint in &mut self.checkpoints[..self.checkpoint_len] {
            if checkpoint.id == id {
                return Ok(checkpoint);
            }
        }

        Err(KernelError::CheckpointNotFound)
    }
}
```

- [x] **Step 2: Register module and remove old methods**

In `crates/agent-kernel-core/src/lib.rs`, add:

```rust
mod checkpoint_store;
```

In `crates/agent-kernel-core/src/core.rs`, delete the old event-only
`checkpoint(...)`, `rollback(...)`, and `resource_event(...)` helper if no other
method uses it.

- [x] **Step 3: Run checkpoint tests**

Run:

```bash
RUSTC="$(rustup which rustc --toolchain nightly)" RUSTDOC="$(rustup which rustdoc --toolchain nightly)" rustup run nightly cargo test -p agent-kernel-core --test checkpoint_store
```

Expected: all checkpoint store tests pass.

## Task 5: Update Existing Core Tests

**Files:**
- Modify: `crates/agent-kernel-core/tests/kernel_core.rs`
- Modify any core test using old generic arity after `rg "KernelCore<"`.

- [x] **Step 1: Strengthen checkpoint assertions**

In `checkpoint_and_rollback_events_are_recorded_in_order`, after rollback add:

```rust
assert_eq!(core.checkpoints().len(), 1);
assert_eq!(
    core.checkpoints()[0].status,
    CheckpointStatus::RollbackRequested
);
```

Import `CheckpointStatus`.

In `checkpoint_requires_checkpoint_capability`, assert:

```rust
assert_eq!(result, Err(KernelError::OperationDenied));
assert!(core.checkpoints().is_empty());
```

- [x] **Step 2: Run core tests**

Run:

```bash
RUSTC="$(rustup which rustc --toolchain nightly)" RUSTDOC="$(rustup which rustdoc --toolchain nightly)" rustup run nightly cargo test -p agent-kernel-core
```

Expected: all core tests pass.

## Task 6: Update Facade, Boot, Supervisor, And README

**Files:**
- Modify: `crates/agent-kernel/src/lib.rs`
- Modify: `crates/agent-kernel/src/scheduler.rs`
- Modify: `crates/agent-kernel/tests/*.rs`
- Modify: `crates/agent-kernel-boot/src/lib.rs`
- Modify: `crates/agent-kernel-boot/tests/boot_flow.rs`
- Modify: `crates/agent-kernel-x86_64/src/main.rs`
- Modify: `crates/agent-supervisor/src/main.rs`
- Modify: `README.md`

- [x] **Step 1: Update facade generic arity and accessor**

In `agent-kernel/src/lib.rs`, add `CheckpointRecord` to imports, add
`CHECKPOINTS` after `OBSERVATIONS` in all `AgentKernel` generics, update the
core field type, and add:

```rust
pub fn checkpoints(&self) -> &[CheckpointRecord] {
    self.core.checkpoints()
}
```

Apply the same arity to `agent-kernel/src/scheduler.rs`.

- [x] **Step 2: Update facade tests**

Insert checkpoint capacity after observation capacity in all `AgentKernel`
aliases. In `kernel_facade.rs`, import `CheckpointStatus` and add to the
checkpoint/rollback syscall test:

```rust
assert_eq!(kernel.checkpoints().len(), 1);
assert_eq!(
    kernel.checkpoints()[0].status,
    CheckpointStatus::RollbackRequested
);
```

- [x] **Step 3: Update boot and x86 arity**

Add `CHECKPOINTS` after `OBSERVATIONS` in `BootedKernel` generics and use:

```rust
BootedKernel::<8, 8, 16, 4, 4, 4, 0, 4, 4>
```

in boot tests and x86 boot entry.

- [x] **Step 4: Update supervisor arity and README**

Use:

```rust
AgentKernel::<8, 8, 32, 8, 8, 8, 8, 8, 8>
```

in `crates/agent-supervisor/src/main.rs`.

In `README.md`, change current behavior bullets:

```text
7. Create and store a checkpoint record.
8. Request rollback for that checkpoint.
```

Keep expected supervisor and QEMU output unchanged.

- [x] **Step 5: Run facade and runtime tests**

Run:

```bash
RUSTC="$(rustup which rustc --toolchain nightly)" RUSTDOC="$(rustup which rustdoc --toolchain nightly)" rustup run nightly cargo test -p agent-kernel
RUSTC="$(rustup which rustc --toolchain nightly)" RUSTDOC="$(rustup which rustdoc --toolchain nightly)" rustup run nightly cargo test -p agent-kernel-boot
RUSTC="$(rustup which rustc --toolchain nightly)" RUSTDOC="$(rustup which rustdoc --toolchain nightly)" rustup run nightly cargo test -p agent-supervisor
```

Expected: all pass.

## Task 7: Full Verification And Publish

**Files:**
- All files changed by Tasks 1-6.

- [x] **Step 1: Run full verification**

Run:

```bash
rustup run nightly cargo fmt --check
RUSTC="$(rustup which rustc --toolchain nightly)" RUSTDOC="$(rustup which rustdoc --toolchain nightly)" rustup run nightly cargo test --workspace
RUSTC="$(rustup which rustc --toolchain nightly)" RUSTDOC="$(rustup which rustdoc --toolchain nightly)" rustup run nightly cargo run -p agent-supervisor
scripts/run-qemu.sh
```

Expected: formatting passes, workspace tests pass, supervisor output still
includes `event[5] checkpoint` and `event[6] rollback`, and QEMU output remains:

```text
AGENT_KERNEL_QEMU_BOOT_OK
event[1] capability_granted
event[2] observation
event[3] action
event[4] verification
SUPERVISOR_HANDOFF_READY
```

- [x] **Step 2: Check boundaries and file sizes**

Run:

```bash
rg -n "\b(Vec|String|Box|HashMap|Rc|Arc|std::|println!|format!|thread|async|await|env::|fs::|net::)\b" \
  crates/agent-kernel-core/src/checkpoint.rs \
  crates/agent-kernel-core/src/checkpoint_store.rs
wc -l crates/agent-kernel-core/src/*.rs crates/agent-kernel/src/*.rs crates/agent-supervisor/src/*.rs crates/agent-kernel-core/tests/*.rs crates/agent-kernel/tests/*.rs crates/agent-supervisor/tests/*.rs crates/agent-kernel-boot/src/*.rs crates/agent-kernel-boot/tests/*.rs crates/agent-kernel-x86_64/src/*.rs 2>/dev/null
git diff --check
git status --short
```

Expected: no unsupported no_std patterns in new core modules, no source file
exceeds hard size limits, diff check passes.

- [x] **Step 3: Commit and push**

Run:

```bash
git add docs/superpowers/specs/2026-07-03-checkpoint-store-v0-design.md docs/superpowers/plans/2026-07-03-checkpoint-store-v0.md crates README.md
git commit -m "feat: add checkpoint store"
git push
```

Expected: push succeeds and `git status --short --branch` shows `main...origin/main`.

## Self-Review

Spec coverage: every data model, API, error, atomicity, facade, boot, supervisor,
README, and verification requirement from the spec maps to a task above.

Placeholder scan: no `TBD`, `TODO`, or open-ended implementation placeholders
remain.

Type consistency: `CheckpointRecord`, `CheckpointStatus`, `checkpoints()`,
`CheckpointStoreFull`, `CheckpointAlreadyExists`, `CheckpointNotFound`,
`CheckpointResourceMismatch`, and `CheckpointStatusMismatch` are used
consistently across tasks.
