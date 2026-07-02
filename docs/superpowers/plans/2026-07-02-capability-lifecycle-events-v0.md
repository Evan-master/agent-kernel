# Capability Lifecycle Events V0 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Record capability grant, derivation, and revocation in the deterministic kernel event log.

**Architecture:** `agent-kernel-core` adds lifecycle event kinds plus `Event` fields for operation sets and source capability parentage. Capability store methods check event capacity before mutating authority state, and task delegation reserves two event slots before deriving task authority and recording delegation. Facade, supervisor, boot, and README traces are updated for the richer event sequence.

**Tech Stack:** Rust nightly, no_std core and facade crates, fixed-capacity arrays, Cargo workspace tests, QEMU BIOS boot script.

---

## File Structure

- Create `crates/agent-kernel-core/tests/capability_lifecycle.rs` for root grant, derive, revoke, and event-capacity behavior.
- Create `crates/agent-kernel/tests/capability_lifecycle.rs` for facade-level `sys_grant` evidence without growing the already large facade test file.
- Modify `crates/agent-kernel-core/src/event.rs` to add lifecycle kinds and event metadata fields.
- Modify `crates/agent-kernel-core/src/event_log.rs` to add a reusable event slot capacity helper.
- Modify `crates/agent-kernel-core/src/capability_store.rs` to record lifecycle events and keep mutation atomic on event capacity errors.
- Modify `crates/agent-kernel-core/src/core.rs`, `task_store.rs`, and `scheduler.rs` to populate the new event fields.
- Modify existing core and facade tests to compare against event counts after visible grants.
- Modify `crates/agent-supervisor/src/main.rs`, its test, x86 serial output, boot tests, and `README.md` for the expanded event sequence.

## Task 1: Core Lifecycle Red Tests

**Files:**
- Create: `crates/agent-kernel-core/tests/capability_lifecycle.rs`

- [ ] **Step 1: Add lifecycle tests**

Create `crates/agent-kernel-core/tests/capability_lifecycle.rs`:

```rust
use agent_kernel_core::{
    AgentId, CapabilityId, EventKind, KernelCore, KernelError, Operation, OperationSet,
    ResourceKind, TaskStatus,
};

type TestCore = KernelCore<4, 8, 32, 6, 4>;

#[test]
fn grant_capability_records_capability_granted_event() {
    let mut core = TestCore::new();
    let agent = AgentId::new(1);
    let resource = core
        .register_resource(ResourceKind::Workspace, None)
        .expect("resource should fit");
    let operations = OperationSet::empty()
        .with(Operation::Observe)
        .with(Operation::Act);

    let capability = core
        .grant_capability(agent, resource, operations)
        .expect("grant should fit");

    let events = core.events();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].kind, EventKind::CapabilityGranted);
    assert_eq!(events[0].agent, agent);
    assert_eq!(events[0].resource, Some(resource));
    assert_eq!(events[0].capability, Some(capability));
    assert_eq!(events[0].source_capability, None);
    assert_eq!(events[0].operations, operations);
    assert_eq!(events[0].task, None);
    assert_eq!(events[0].target_agent, None);
}

#[test]
fn grant_capability_returns_event_log_full_without_allocating() {
    let mut core = KernelCore::<1, 1, 0, 0, 0>::new();
    let agent = AgentId::new(2);
    let resource = core
        .register_resource(ResourceKind::Workspace, None)
        .expect("resource should fit");

    let result = core.grant_capability(agent, resource, OperationSet::only(Operation::Observe));

    assert_eq!(result, Err(KernelError::EventLogFull));
    assert!(core.events().is_empty());
    assert_eq!(
        core.authorize(
            agent,
            CapabilityId::new(1),
            resource,
            Operation::Observe
        ),
        Err(KernelError::CapabilityNotFound)
    );
}

#[test]
fn revoke_capability_records_capability_revoked_event() {
    let mut core = TestCore::new();
    let agent = AgentId::new(3);
    let resource = core
        .register_resource(ResourceKind::Service, None)
        .expect("resource should fit");
    let operations = OperationSet::only(Operation::Observe);
    let capability = core
        .grant_capability(agent, resource, operations)
        .expect("grant should fit");

    core.revoke_capability(capability)
        .expect("capability should revoke");

    let events = core.events();
    assert_eq!(events.len(), 2);
    assert_eq!(events[1].kind, EventKind::CapabilityRevoked);
    assert_eq!(events[1].agent, agent);
    assert_eq!(events[1].resource, Some(resource));
    assert_eq!(events[1].capability, Some(capability));
    assert_eq!(events[1].operations, operations);
    assert_eq!(events[1].source_capability, None);
    assert_eq!(
        core.authorize(agent, capability, resource, Operation::Observe),
        Err(KernelError::CapabilityRevoked)
    );
    assert_eq!(core.events().len(), 2);
}

#[test]
fn revoke_capability_returns_event_log_full_without_revoking() {
    let mut core = KernelCore::<1, 1, 1, 0, 0>::new();
    let agent = AgentId::new(4);
    let resource = core
        .register_resource(ResourceKind::Workspace, None)
        .expect("resource should fit");
    let capability = core
        .grant_capability(agent, resource, OperationSet::only(Operation::Observe))
        .expect("grant should consume only event slot");

    let result = core.revoke_capability(capability);

    assert_eq!(result, Err(KernelError::EventLogFull));
    assert_eq!(
        core.authorize(agent, capability, resource, Operation::Act),
        Err(KernelError::OperationDenied)
    );
    assert_eq!(core.events().len(), 1);
}

#[test]
fn delegate_task_records_capability_derived_before_delegation() {
    let mut core = TestCore::new();
    let owner = AgentId::new(5);
    let assignee = AgentId::new(6);
    let resource = core
        .register_resource(ResourceKind::Workspace, None)
        .expect("resource should fit");
    let owner_capability = core
        .grant_capability(
            owner,
            resource,
            OperationSet::empty()
                .with(Operation::Act)
                .with(Operation::Delegate),
        )
        .expect("owner capability should fit");
    let task = core
        .create_task(owner, owner_capability, resource)
        .expect("task should be created");

    let delegation = core
        .delegate_task(owner, owner_capability, task, assignee)
        .expect("task should delegate");
    let derived = core.tasks()[0]
        .delegated_capability
        .expect("delegation should derive capability");

    let events = core.events();
    assert_eq!(events[2].kind, EventKind::CapabilityDerived);
    assert_eq!(events[2].agent, owner);
    assert_eq!(events[2].target_agent, Some(assignee));
    assert_eq!(events[2].resource, Some(resource));
    assert_eq!(events[2].capability, Some(derived));
    assert_eq!(events[2].source_capability, Some(owner_capability));
    assert_eq!(events[2].operations, OperationSet::only(Operation::Act));
    assert_eq!(events[2].task, Some(task));
    assert_eq!(events[3].kind, EventKind::DelegationRequested);
    assert_eq!(delegation.capability, Some(derived));
}

#[test]
fn delegate_task_requires_two_event_slots_for_derive_and_delegation() {
    let mut core = KernelCore::<1, 4, 3, 2, 2>::new();
    let owner = AgentId::new(7);
    let assignee = AgentId::new(8);
    let resource = core
        .register_resource(ResourceKind::Workspace, None)
        .expect("resource should fit");
    let owner_capability = core
        .grant_capability(
            owner,
            resource,
            OperationSet::empty()
                .with(Operation::Act)
                .with(Operation::Delegate),
        )
        .expect("owner capability should fit");
    let task = core
        .create_task(owner, owner_capability, resource)
        .expect("task should be created");
    let events_after_create = core.events().len();

    let result = core.delegate_task(owner, owner_capability, task, assignee);

    assert_eq!(result, Err(KernelError::EventLogFull));
    assert_eq!(core.tasks()[0].status, TaskStatus::Created);
    assert_eq!(core.tasks()[0].assignee, None);
    assert_eq!(core.tasks()[0].delegated_capability, None);
    assert_eq!(core.events().len(), events_after_create);
}
```

- [ ] **Step 2: Run the red test**

Run:

```bash
PATH="$HOME/.cargo/bin:$PATH" RUSTC="$(rustup which rustc --toolchain nightly)" rustup run nightly cargo test -p agent-kernel-core --test capability_lifecycle
```

Expected: compile failures for missing `EventKind::CapabilityGranted`, `EventKind::CapabilityDerived`, `EventKind::CapabilityRevoked`, `Event::source_capability`, and `Event::operations`.

## Task 2: Core Lifecycle Implementation

**Files:**
- Modify: `crates/agent-kernel-core/src/event.rs`
- Modify: `crates/agent-kernel-core/src/event_log.rs`
- Modify: `crates/agent-kernel-core/src/capability_store.rs`
- Modify: `crates/agent-kernel-core/src/core.rs`
- Modify: `crates/agent-kernel-core/src/task_store.rs`
- Modify: `crates/agent-kernel-core/src/scheduler.rs`

- [ ] **Step 1: Extend event types**

In `crates/agent-kernel-core/src/event.rs`, import `OperationSet`, add lifecycle variants to `EventKind`, and add these fields to `Event` plus `Event::empty()`:

```rust
pub source_capability: Option<CapabilityId>,
pub operations: OperationSet,
```

Set `source_capability: None` and `operations: OperationSet::empty()` for the empty event.

- [ ] **Step 2: Add event slot helper**

In `crates/agent-kernel-core/src/event_log.rs`, add:

```rust
pub(crate) fn ensure_event_slots(&self, needed: usize) -> Result<(), KernelError> {
    if EVENTS.saturating_sub(self.event_len) < needed {
        Err(KernelError::EventLogFull)
    } else {
        Ok(())
    }
}
```

- [ ] **Step 3: Record capability lifecycle events**

In `crates/agent-kernel-core/src/capability_store.rs`:

- find capability slots by index before mutation so no mutable borrow is held while recording,
- call `ensure_event_slots(1)` before allocating or revoking,
- record `CapabilityGranted` after root allocation,
- record `CapabilityDerived` after task-scoped allocation,
- record `CapabilityRevoked` after setting `revoked = true`.

Use a private helper shaped like:

```rust
fn record_capability_event(
    &mut self,
    kind: EventKind,
    agent: AgentId,
    resource: ResourceId,
    capability: CapabilityId,
    source_capability: Option<CapabilityId>,
    operations: OperationSet,
    task: Option<TaskId>,
    target_agent: Option<AgentId>,
) -> Result<Event, KernelError> {
    self.record(Event {
        sequence: self.next_sequence,
        agent,
        kind,
        resource: Some(resource),
        capability: Some(capability),
        source_capability,
        action: None,
        operation: None,
        operations,
        checkpoint: None,
        task,
        target_agent,
    })
}
```

- [ ] **Step 4: Populate new event fields elsewhere**

In `core.rs`, `task_store.rs`, and `scheduler.rs`, add
`source_capability: None` and `operations: OperationSet::empty()` to existing
non-lifecycle `Event` literals.

In `task_store.rs`, replace the single event-capacity check in `delegate_task`
with `self.ensure_event_slots(2)?` before `derive_task_capability`.

- [ ] **Step 5: Run focused lifecycle test**

Run:

```bash
PATH="$HOME/.cargo/bin:$PATH" RUSTC="$(rustup which rustc --toolchain nightly)" rustup run nightly cargo test -p agent-kernel-core --test capability_lifecycle
```

Expected: all lifecycle tests pass.

## Task 3: Update Existing Tests And Runtime Output

**Files:**
- Create: `crates/agent-kernel/tests/capability_lifecycle.rs`
- Modify: existing tests under `crates/agent-kernel-core/tests/`
- Modify: `crates/agent-kernel/tests/kernel_facade.rs`
- Modify: `crates/agent-kernel-boot/tests/boot_flow.rs`
- Modify: `crates/agent-supervisor/src/main.rs`
- Modify: `crates/agent-supervisor/tests/supervisor_flow.rs`
- Modify: `crates/agent-kernel-x86_64/src/main.rs`
- Modify: `README.md`

- [ ] **Step 1: Add facade lifecycle test**

Create `crates/agent-kernel/tests/capability_lifecycle.rs`:

```rust
use agent_kernel::AgentKernel;
use agent_kernel_core::{AgentId, EventKind, Operation, OperationSet, ResourceKind};

type TestKernel = AgentKernel<2, 2, 4, 1, 1>;

#[test]
fn sys_grant_records_capability_granted_event() {
    let mut kernel = TestKernel::new();
    let agent = AgentId::new(1);
    let resource = kernel
        .sys_register_resource(ResourceKind::Workspace, None)
        .expect("resource should fit");
    let operations = OperationSet::empty()
        .with(Operation::Observe)
        .with(Operation::Act);

    let capability = kernel
        .sys_grant(agent, resource, operations)
        .expect("grant should fit");

    let events = kernel.events();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].kind, EventKind::CapabilityGranted);
    assert_eq!(events[0].agent, agent);
    assert_eq!(events[0].resource, Some(resource));
    assert_eq!(events[0].capability, Some(capability));
    assert_eq!(events[0].operations, operations);
}
```

- [ ] **Step 2: Update existing test expectations**

For existing tests that perform grants, change event-count assertions to capture
`let events_before = ...` immediately before the operation being tested.

For lifecycle-order assertions, include visible grant and derived events. The
full task lifecycle sequence becomes:

```rust
assert_eq!(kernel.events()[0].kind, EventKind::CapabilityGranted);
assert_eq!(kernel.events()[1].kind, EventKind::TaskCreated);
assert_eq!(kernel.events()[2].kind, EventKind::CapabilityDerived);
assert_eq!(kernel.events()[3].kind, EventKind::DelegationRequested);
assert_eq!(kernel.events()[4].kind, EventKind::TaskAccepted);
assert_eq!(kernel.events()[5].kind, EventKind::TaskQueued);
assert_eq!(kernel.events()[6].kind, EventKind::TaskDispatched);
assert_eq!(kernel.events()[7].kind, EventKind::TaskCompleted);
assert_eq!(kernel.events()[8].kind, EventKind::TaskVerified);
```

- [ ] **Step 3: Update supervisor formatter**

In `crates/agent-supervisor/src/main.rs`, add match arms:

```rust
EventKind::CapabilityGranted => format_capability_event(event, "capability_granted"),
EventKind::CapabilityDerived => format_capability_event(event, "capability_derived"),
EventKind::CapabilityRevoked => format_capability_event(event, "capability_revoked"),
```

Add:

```rust
fn format_capability_event(event: &Event, label: &str) -> String {
    let agent = event.agent.raw();
    let resource = event
        .resource
        .map(|resource| resource.raw())
        .unwrap_or_default();
    let capability = event
        .capability
        .map(|capability| capability.raw())
        .unwrap_or_default();

    format!(
        "event[{}] {} agent={} resource={} capability={}",
        event.sequence, label, agent, resource, capability
    )
}
```

The supervisor sequence becomes:

```text
event[1] capability_granted agent=1 resource=1 capability=1
event[2] observation agent=1 resource=1
event[3] action agent=1 resource=1 action=1
event[4] verification agent=1 resource=1 action=1
event[5] checkpoint agent=1 resource=1 checkpoint=1
event[6] rollback agent=1 resource=1 checkpoint=1
event[7] task_created agent=1 resource=1 task=1
event[8] capability_derived agent=1 resource=1 capability=2
event[9] delegation agent=1 resource=1 task=1 target_agent=2
event[10] task_accepted agent=2 resource=1 task=1
event[11] task_queued agent=2 resource=1 task=1
event[12] task_dispatched agent=2 resource=1 task=1
event[13] task_completed agent=2 resource=1 task=1
event[14] task_verified agent=1 resource=1 task=1
```

- [ ] **Step 4: Update boot serial output**

In `crates/agent-kernel-x86_64/src/main.rs`, add labels for
`CapabilityGranted`, `CapabilityDerived`, and `CapabilityRevoked`.

Update `crates/agent-kernel-boot/tests/boot_flow.rs` to expect four events:

```rust
assert_eq!(events.len(), 4);
assert_eq!(events[0].kind, EventKind::CapabilityGranted);
assert_eq!(events[1].kind, EventKind::Observation);
assert_eq!(events[2].kind, EventKind::ActionExecuted);
assert_eq!(events[3].kind, EventKind::VerificationRequested);
```

- [ ] **Step 5: Update README traces**

Update `README.md` current behavior and expected output blocks so capability
grant and derived events are listed in the event sequence.

- [ ] **Step 6: Run workspace tests**

Run:

```bash
PATH="$HOME/.cargo/bin:$PATH" RUSTC="$(rustup which rustc --toolchain nightly)" rustup run nightly cargo test --workspace
```

Expected: all tests pass.

## Task 4: Verification, Commit, And Push

**Files:**
- All files changed by Tasks 1-3.

- [ ] **Step 1: Run formatting**

Run:

```bash
PATH="$HOME/.cargo/bin:$PATH" rustup run nightly cargo fmt --check
```

Expected: command exits successfully.

- [ ] **Step 2: Run full workspace tests**

Run:

```bash
PATH="$HOME/.cargo/bin:$PATH" RUSTC="$(rustup which rustc --toolchain nightly)" rustup run nightly cargo test --workspace
```

Expected: command exits successfully.

- [ ] **Step 3: Run supervisor flow**

Run:

```bash
PATH="$HOME/.cargo/bin:$PATH" RUSTC="$(rustup which rustc --toolchain nightly)" rustup run nightly cargo run -p agent-supervisor
```

Expected: output includes `capability_granted`, `capability_derived`, and the
final `task_verified` event.

- [ ] **Step 4: Run QEMU boot**

Run:

```bash
scripts/run-qemu.sh
```

Expected: serial output includes `AGENT_KERNEL_QEMU_BOOT_OK`,
`event[1] capability_granted`, `event[4] verification`, and
`SUPERVISOR_HANDOFF_READY`.

- [ ] **Step 5: Commit and push**

Run:

```bash
git status --short
git add README.md \
  crates/agent-kernel-core/src/event.rs \
  crates/agent-kernel-core/src/event_log.rs \
  crates/agent-kernel-core/src/capability_store.rs \
  crates/agent-kernel-core/src/core.rs \
  crates/agent-kernel-core/src/task_store.rs \
  crates/agent-kernel-core/src/scheduler.rs \
  crates/agent-kernel-core/tests/capability_lifecycle.rs \
  crates/agent-kernel-core/tests/kernel_core.rs \
  crates/agent-kernel-core/tests/delegated_capability.rs \
  crates/agent-kernel-core/tests/task_authority.rs \
  crates/agent-kernel-core/tests/task_lifecycle.rs \
  crates/agent-kernel-core/tests/scheduler.rs \
  crates/agent-kernel-core/tests/capability_revocation.rs \
  crates/agent-kernel/tests/capability_lifecycle.rs \
  crates/agent-kernel/tests/kernel_facade.rs \
  crates/agent-kernel-boot/tests/boot_flow.rs \
  crates/agent-kernel-x86_64/src/main.rs \
  crates/agent-supervisor/src/main.rs \
  crates/agent-supervisor/tests/supervisor_flow.rs \
  docs/superpowers/plans/2026-07-02-capability-lifecycle-events-v0.md
git commit -m "feat: record capability lifecycle events"
git push
```

Expected: commit is created and pushed to `origin/main`.

## Self-Review

Spec coverage:

- Root grant events are covered by Task 1 and Task 2.
- Derived capability events are covered by Task 1 and Task 2.
- Revoke events and event-capacity atomicity are covered by Task 1 and Task 2.
- Facade, supervisor, boot, README, and QEMU compatibility updates are covered by Task 3.
- Full verification commands are covered by Task 4.

Placeholder scan:

- The plan contains complete file paths, concrete tests, commands, and expected outcomes.

Type consistency:

- `EventKind`, `OperationSet`, `source_capability`, and `operations` match the spec names.
