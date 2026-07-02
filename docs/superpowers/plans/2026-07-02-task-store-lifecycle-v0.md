# Task Store Lifecycle V0 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a kernel-owned fixed-capacity task store with lifecycle transitions for create, delegate, accept, complete, verify, and cancel.

**Architecture:** `agent-kernel-core` owns task state and transition validation. `agent-kernel` exposes syscall-style wrappers only. `agent-supervisor` demonstrates the flow through the facade without mutating internals.

**Tech Stack:** Rust nightly, no_std core/facade crates, fixed-capacity arrays, existing Cargo workspace, QEMU boot script.

---

## File Structure

- Create `crates/agent-kernel-core/src/task.rs` for `Task` and `TaskStatus`.
- Create `crates/agent-kernel-core/src/task_store.rs` for task lookup, allocation, transition validation, and task lifecycle methods.
- Modify `crates/agent-kernel-core/src/core.rs` to add `TASKS`, task storage fields, `next_task`, and remove the old event-only `delegate` method.
- Modify `crates/agent-kernel-core/src/event.rs` to add task lifecycle event kinds.
- Modify `crates/agent-kernel-core/src/error.rs` to add task errors.
- Modify `crates/agent-kernel-core/src/lib.rs` to export task types and modules.
- Modify `crates/agent-kernel-core/src/authorization.rs`, `lookup.rs`, and `event_log.rs` to accept the new `TASKS` const parameter.
- Modify `crates/agent-kernel/src/lib.rs` to mirror `TASKS` and expose task syscalls.
- Modify boot/x86/supervisor/tests to use the four-parameter kernel type.
- Modify `README.md` to document task store lifecycle output.

## Task 1: Core Red Tests

**Files:**
- Modify: `crates/agent-kernel-core/tests/kernel_core.rs`

- [ ] **Step 1: Update imports and kernel type aliases**

Add:

```rust
use agent_kernel_core::{TaskStatus, KernelError};

type TestCore = KernelCore<4, 4, 16, 4>;
```

Then replace `KernelCore::<4, 4, 8>::new()` with `TestCore::new()` in existing core tests.

- [ ] **Step 2: Add failing task lifecycle tests**

Append tests with these exact behaviors:

```rust
#[test]
fn create_task_allocates_kernel_task_and_records_event() {
    let mut core = TestCore::new();
    let agent = AgentId::new(11);
    let resource = core.register_resource(ResourceKind::Workspace, None).unwrap();
    let capability = core
        .grant_capability(agent, resource, OperationSet::only(Operation::Act))
        .unwrap();

    let task = core.create_task(agent, capability, resource).unwrap();

    assert_eq!(task, TaskId::new(1));
    assert_eq!(core.tasks().len(), 1);
    assert_eq!(core.tasks()[0].id, task);
    assert_eq!(core.tasks()[0].owner, agent);
    assert_eq!(core.tasks()[0].resource, resource);
    assert_eq!(core.tasks()[0].assignee, None);
    assert_eq!(core.tasks()[0].status, TaskStatus::Created);
    assert_eq!(core.events()[0].kind, EventKind::TaskCreated);
    assert_eq!(core.events()[0].task, Some(task));
}

#[test]
fn task_lifecycle_reaches_verified_through_authorized_transitions() {
    let mut core = TestCore::new();
    let owner = AgentId::new(12);
    let assignee = AgentId::new(13);
    let resource = core.register_resource(ResourceKind::Workspace, None).unwrap();
    let owner_capability = core
        .grant_capability(
            owner,
            resource,
            OperationSet::empty()
                .with(Operation::Act)
                .with(Operation::Delegate)
                .with(Operation::Verify),
        )
        .unwrap();
    let assignee_capability = core
        .grant_capability(assignee, resource, OperationSet::only(Operation::Act))
        .unwrap();

    let task = core.create_task(owner, owner_capability, resource).unwrap();
    core.delegate_task(owner, owner_capability, task, assignee).unwrap();
    core.accept_task(assignee, task).unwrap();
    core.complete_task(assignee, assignee_capability, task).unwrap();
    core.verify_task(owner, owner_capability, task).unwrap();

    assert_eq!(core.tasks()[0].status, TaskStatus::Verified);
    assert_eq!(core.events()[0].kind, EventKind::TaskCreated);
    assert_eq!(core.events()[1].kind, EventKind::DelegationRequested);
    assert_eq!(core.events()[2].kind, EventKind::TaskAccepted);
    assert_eq!(core.events()[3].kind, EventKind::TaskCompleted);
    assert_eq!(core.events()[4].kind, EventKind::TaskVerified);
}
```

- [ ] **Step 3: Add failing negative tests**

Append:

```rust
#[test]
fn task_operations_reject_invalid_authority_and_status_without_events() {
    let mut core = TestCore::new();
    let owner = AgentId::new(14);
    let wrong_agent = AgentId::new(15);
    let resource = core.register_resource(ResourceKind::Workspace, None).unwrap();
    let owner_capability = core
        .grant_capability(
            owner,
            resource,
            OperationSet::empty().with(Operation::Act).with(Operation::Delegate),
        )
        .unwrap();
    let wrong_capability = core
        .grant_capability(wrong_agent, resource, OperationSet::only(Operation::Observe))
        .unwrap();
    let task = core.create_task(owner, owner_capability, resource).unwrap();
    let events_after_create = core.events().len();

    assert_eq!(
        core.delegate_task(owner, wrong_capability, task, wrong_agent),
        Err(KernelError::AgentMismatch)
    );
    assert_eq!(core.accept_task(wrong_agent, task), Err(KernelError::TaskAgentMismatch));
    assert_eq!(
        core.complete_task(owner, owner_capability, task),
        Err(KernelError::TaskStatusMismatch)
    );
    assert_eq!(core.tasks()[0].status, TaskStatus::Created);
    assert_eq!(core.events().len(), events_after_create);
}

#[test]
fn task_store_capacity_returns_task_store_full() {
    let mut core = KernelCore::<4, 4, 8, 1>::new();
    let agent = AgentId::new(16);
    let resource = core.register_resource(ResourceKind::Workspace, None).unwrap();
    let capability = core
        .grant_capability(agent, resource, OperationSet::only(Operation::Act))
        .unwrap();

    core.create_task(agent, capability, resource).unwrap();
    let result = core.create_task(agent, capability, resource);

    assert_eq!(result, Err(KernelError::TaskStoreFull));
    assert_eq!(core.tasks().len(), 1);
}
```

- [ ] **Step 4: Run red tests**

Run:

```bash
PATH="$HOME/.cargo/bin:$PATH" RUSTC="$(rustup which rustc --toolchain nightly)" rustup run nightly cargo test -p agent-kernel-core task
```

Expected: compile failures for missing `TaskStatus`, `create_task`, `delegate_task`, `accept_task`, `complete_task`, `verify_task`, and `tasks`.

## Task 2: Core Task Model And Store

**Files:**
- Create: `crates/agent-kernel-core/src/task.rs`
- Create: `crates/agent-kernel-core/src/task_store.rs`
- Modify: `crates/agent-kernel-core/src/core.rs`
- Modify: `crates/agent-kernel-core/src/error.rs`
- Modify: `crates/agent-kernel-core/src/event.rs`
- Modify: `crates/agent-kernel-core/src/lib.rs`
- Modify: `crates/agent-kernel-core/src/authorization.rs`
- Modify: `crates/agent-kernel-core/src/event_log.rs`
- Modify: `crates/agent-kernel-core/src/lookup.rs`

- [ ] **Step 1: Add task model**

Create `task.rs`:

```rust
//! Kernel-owned task model.
//!
//! This module belongs to `agent-kernel-core`. It defines copyable task state
//! for the fixed-capacity no_std task store. It has no host dependencies and no
//! allocation.

use crate::{AgentId, ResourceId, TaskId};

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum TaskStatus {
    Created,
    Delegated,
    Accepted,
    Completed,
    Verified,
    Cancelled,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Task {
    pub id: TaskId,
    pub owner: AgentId,
    pub resource: ResourceId,
    pub assignee: Option<AgentId>,
    pub status: TaskStatus,
}

impl Task {
    pub(crate) const fn empty() -> Self {
        Self {
            id: TaskId::new(0),
            owner: AgentId::new(0),
            resource: ResourceId::new(0),
            assignee: None,
            status: TaskStatus::Cancelled,
        }
    }
}
```

- [ ] **Step 2: Add task errors and event kinds**

In `error.rs`, add:

```rust
TaskStoreFull,
TaskNotFound,
TaskAgentMismatch,
TaskStatusMismatch,
```

In `event.rs`, add:

```rust
TaskCreated,
TaskAccepted,
TaskCompleted,
TaskVerified,
TaskCancelled,
```

- [ ] **Step 3: Add task storage to `KernelCore`**

Change the type to:

```rust
pub struct KernelCore<
    const RESOURCES: usize,
    const CAPS: usize,
    const EVENTS: usize,
    const TASKS: usize,
> {
    pub(crate) resources: [Option<Resource>; RESOURCES],
    pub(crate) capabilities: [Option<Capability>; CAPS],
    pub(crate) events: [Event; EVENTS],
    pub(crate) tasks: [Task; TASKS],
    pub(crate) event_len: usize,
    pub(crate) task_len: usize,
    pub(crate) next_resource: u64,
    pub(crate) next_capability: u64,
    pub(crate) next_task: u64,
    pub(crate) next_sequence: u64,
}
```

Initialize with:

```rust
tasks: [Task::empty(); TASKS],
task_len: 0,
next_task: 1,
```

- [ ] **Step 4: Update helper impl generics**

Update impl headers in `authorization.rs`, `event_log.rs`, and `lookup.rs` to:

```rust
impl<const RESOURCES: usize, const CAPS: usize, const EVENTS: usize, const TASKS: usize>
    KernelCore<RESOURCES, CAPS, EVENTS, TASKS>
```

- [ ] **Step 5: Implement task store**

Create `task_store.rs` with methods:

```rust
//! Fixed-capacity task store and lifecycle transitions.
//!
//! This module belongs to `agent-kernel-core`. It owns task allocation,
//! lifecycle validation, capability-gated task mutation, and task event
//! recording. It performs no allocation or host I/O.

use crate::{
    AgentId, CapabilityId, Event, EventKind, KernelCore, KernelError, Operation, ResourceId, Task,
    TaskId, TaskStatus,
};

impl<const RESOURCES: usize, const CAPS: usize, const EVENTS: usize, const TASKS: usize>
    KernelCore<RESOURCES, CAPS, EVENTS, TASKS>
{
    pub fn create_task(
        &mut self,
        agent: AgentId,
        capability: CapabilityId,
        resource: ResourceId,
    ) -> Result<TaskId, KernelError> {
        self.ensure_authorized(agent, capability, resource, Operation::Act)?;
        if self.task_len >= TASKS {
            return Err(KernelError::TaskStoreFull);
        }

        let task = TaskId::new(self.next_task);
        self.next_task += 1;
        self.tasks[self.task_len] = Task {
            id: task,
            owner: agent,
            resource,
            assignee: None,
            status: TaskStatus::Created,
        };
        self.task_len += 1;
        self.record_task_event(EventKind::TaskCreated, agent, Some(capability), task, None)?;
        Ok(task)
    }

    pub fn delegate_task(
        &mut self,
        agent: AgentId,
        capability: CapabilityId,
        task: TaskId,
        target_agent: AgentId,
    ) -> Result<Event, KernelError> {
        let current = self.find_task(task)?;
        self.ensure_authorized(agent, capability, current.resource, Operation::Delegate)?;
        ensure_status(current.status, &[TaskStatus::Created])?;
        let task_ref = self.find_task_mut(task)?;
        task_ref.assignee = Some(target_agent);
        task_ref.status = TaskStatus::Delegated;
        self.record_task_event(
            EventKind::DelegationRequested,
            agent,
            Some(capability),
            task,
            Some(target_agent),
        )
    }

    pub fn accept_task(&mut self, agent: AgentId, task: TaskId) -> Result<Event, KernelError> {
        let current = self.find_task(task)?;
        ensure_status(current.status, &[TaskStatus::Delegated])?;
        if current.assignee != Some(agent) {
            return Err(KernelError::TaskAgentMismatch);
        }
        self.find_task_mut(task)?.status = TaskStatus::Accepted;
        self.record_task_event(EventKind::TaskAccepted, agent, None, task, None)
    }

    pub fn complete_task(
        &mut self,
        agent: AgentId,
        capability: CapabilityId,
        task: TaskId,
    ) -> Result<Event, KernelError> {
        let current = self.find_task(task)?;
        self.ensure_authorized(agent, capability, current.resource, Operation::Act)?;
        ensure_status(current.status, &[TaskStatus::Accepted])?;
        if current.assignee != Some(agent) {
            return Err(KernelError::TaskAgentMismatch);
        }
        self.find_task_mut(task)?.status = TaskStatus::Completed;
        self.record_task_event(EventKind::TaskCompleted, agent, Some(capability), task, None)
    }

    pub fn verify_task(
        &mut self,
        agent: AgentId,
        capability: CapabilityId,
        task: TaskId,
    ) -> Result<Event, KernelError> {
        let current = self.find_task(task)?;
        self.ensure_authorized(agent, capability, current.resource, Operation::Verify)?;
        ensure_status(current.status, &[TaskStatus::Completed])?;
        self.find_task_mut(task)?.status = TaskStatus::Verified;
        self.record_task_event(EventKind::TaskVerified, agent, Some(capability), task, None)
    }

    pub fn cancel_task(
        &mut self,
        agent: AgentId,
        capability: CapabilityId,
        task: TaskId,
    ) -> Result<Event, KernelError> {
        let current = self.find_task(task)?;
        self.ensure_authorized(agent, capability, current.resource, Operation::Rollback)?;
        ensure_status(
            current.status,
            &[TaskStatus::Created, TaskStatus::Delegated, TaskStatus::Accepted, TaskStatus::Completed],
        )?;
        self.find_task_mut(task)?.status = TaskStatus::Cancelled;
        self.record_task_event(EventKind::TaskCancelled, agent, Some(capability), task, None)
    }

    pub fn tasks(&self) -> &[Task] {
        &self.tasks[..self.task_len]
    }

    pub(crate) fn find_task(&self, id: TaskId) -> Result<Task, KernelError> {
        self.tasks()
            .iter()
            .find(|task| task.id == id)
            .copied()
            .ok_or(KernelError::TaskNotFound)
    }

    pub(crate) fn find_task_mut(&mut self, id: TaskId) -> Result<&mut Task, KernelError> {
        self.tasks[..self.task_len]
            .iter_mut()
            .find(|task| task.id == id)
            .ok_or(KernelError::TaskNotFound)
    }

    fn record_task_event(
        &mut self,
        kind: EventKind,
        agent: AgentId,
        capability: Option<CapabilityId>,
        task: TaskId,
        target_agent: Option<AgentId>,
    ) -> Result<Event, KernelError> {
        let task_record = self.find_task(task)?;
        self.record(Event {
            sequence: self.next_sequence,
            agent,
            kind,
            resource: Some(task_record.resource),
            capability,
            action: None,
            operation: None,
            checkpoint: None,
            task: Some(task),
            target_agent,
        })
    }
}

fn ensure_status(current: TaskStatus, allowed: &[TaskStatus]) -> Result<(), KernelError> {
    if allowed.iter().any(|status| *status == current) {
        Ok(())
    } else {
        Err(KernelError::TaskStatusMismatch)
    }
}
```

- [ ] **Step 6: Export modules and types**

In `lib.rs`, add:

```rust
mod task;
mod task_store;
pub use task::{Task, TaskStatus};
```

- [ ] **Step 7: Run core tests**

Run:

```bash
PATH="$HOME/.cargo/bin:$PATH" RUSTC="$(rustup which rustc --toolchain nightly)" rustup run nightly cargo test -p agent-kernel-core task
```

Expected: task tests pass after compile fixes.

## Task 3: Facade, Boot, And Existing Tests

**Files:**
- Modify: `crates/agent-kernel/src/lib.rs`
- Modify: `crates/agent-kernel/tests/kernel_facade.rs`
- Modify: `crates/agent-kernel-boot/src/lib.rs`
- Modify: `crates/agent-kernel-boot/tests/boot_flow.rs`
- Modify: `crates/agent-kernel-x86_64/src/main.rs`
- Modify: `crates/agent-kernel-core/tests/kernel_core.rs`

- [ ] **Step 1: Mirror task capacity in facade**

Change:

```rust
pub struct AgentKernel<const RESOURCES: usize, const CAPS: usize, const EVENTS: usize, const TASKS: usize> {
    core: KernelCore<RESOURCES, CAPS, EVENTS, TASKS>,
}
```

Update impl and `Default` generics the same way.

- [ ] **Step 2: Add task syscalls**

Add:

```rust
pub fn sys_create_task(
    &mut self,
    agent: AgentId,
    capability: CapabilityId,
    resource: ResourceId,
) -> Result<TaskId, KernelError> {
    self.core.create_task(agent, capability, resource)
}

pub fn sys_delegate_task(
    &mut self,
    agent: AgentId,
    capability: CapabilityId,
    task: TaskId,
    target_agent: AgentId,
) -> Result<Event, KernelError> {
    self.core.delegate_task(agent, capability, task, target_agent)
}

pub fn sys_accept_task(&mut self, agent: AgentId, task: TaskId) -> Result<Event, KernelError> {
    self.core.accept_task(agent, task)
}

pub fn sys_complete_task(
    &mut self,
    agent: AgentId,
    capability: CapabilityId,
    task: TaskId,
) -> Result<Event, KernelError> {
    self.core.complete_task(agent, capability, task)
}

pub fn sys_verify_task(
    &mut self,
    agent: AgentId,
    capability: CapabilityId,
    task: TaskId,
) -> Result<Event, KernelError> {
    self.core.verify_task(agent, capability, task)
}

pub fn sys_cancel_task(
    &mut self,
    agent: AgentId,
    capability: CapabilityId,
    task: TaskId,
) -> Result<Event, KernelError> {
    self.core.cancel_task(agent, capability, task)
}

pub fn tasks(&self) -> &[Task] {
    self.core.tasks()
}
```

- [ ] **Step 3: Update facade tests**

Use `AgentKernel::<4, 4, 16, 4>::new()` everywhere. Replace the old
`delegate_syscall_records_task_delegation` test so it creates a task first and
then calls `sys_delegate_task`.

Add a full syscall lifecycle test using `sys_create_task`, `sys_delegate_task`,
`sys_accept_task`, `sys_complete_task`, and `sys_verify_task`.

- [ ] **Step 4: Update boot and x86 generics**

Use `BootedKernel<const RESOURCES, const CAPS, const EVENTS, const TASKS>` and
`AgentKernel<RESOURCES, CAPS, EVENTS, TASKS>`.

Update boot tests and x86 entry from `::<8, 8, 16>` to `::<8, 8, 16, 4>`.

Update x86 event match to include new task event kinds with serial labels:

```rust
EventKind::TaskCreated => serial_write_line("task_created"),
EventKind::TaskAccepted => serial_write_line("task_accepted"),
EventKind::TaskCompleted => serial_write_line("task_completed"),
EventKind::TaskVerified => serial_write_line("task_verified"),
EventKind::TaskCancelled => serial_write_line("task_cancelled"),
```

- [ ] **Step 5: Run workspace tests**

Run:

```bash
PATH="$HOME/.cargo/bin:$PATH" RUSTC="$(rustup which rustc --toolchain nightly)" rustup run nightly cargo test --workspace
```

Expected: all tests pass after updating generic arity and event matches.

## Task 4: Supervisor Flow And README

**Files:**
- Modify: `crates/agent-supervisor/src/main.rs`
- Modify: `crates/agent-supervisor/tests/supervisor_flow.rs`
- Modify: `README.md`

- [ ] **Step 1: Update supervisor flow**

Use `AgentKernel::<8, 8, 16, 8>::new()`.

Create owner and assignee capabilities:

```rust
let owner_capability = kernel.sys_grant(
    agent,
    workspace,
    OperationSet::empty()
        .with(Operation::Observe)
        .with(Operation::Act)
        .with(Operation::Verify)
        .with(Operation::Checkpoint)
        .with(Operation::Rollback)
        .with(Operation::Delegate),
)?;
let assignee_capability = kernel.sys_grant(
    target_agent,
    workspace,
    OperationSet::only(Operation::Act),
)?;
```

Call task syscalls:

```rust
let task = kernel.sys_create_task(agent, owner_capability, workspace)?;
kernel.sys_delegate_task(agent, owner_capability, task, target_agent)?;
kernel.sys_accept_task(target_agent, task)?;
kernel.sys_complete_task(target_agent, assignee_capability, task)?;
kernel.sys_verify_task(agent, owner_capability, task)?;
```

- [ ] **Step 2: Print task event kinds**

Add format branches:

```rust
EventKind::TaskCreated => "task_created"
EventKind::TaskAccepted => "task_accepted"
EventKind::TaskCompleted => "task_completed"
EventKind::TaskVerified => "task_verified"
EventKind::TaskCancelled => "task_cancelled"
```

Each printed task lifecycle event must include `agent`, `resource`, and `task`.
Delegation must include `target_agent`.

- [ ] **Step 3: Update supervisor test**

Expect output:

```text
event[6] task_created agent=1 resource=1 task=1
event[7] delegation agent=1 resource=1 task=1 target_agent=2
event[8] task_accepted agent=2 resource=1 task=1
event[9] task_completed agent=2 resource=1 task=1
event[10] task_verified agent=1 resource=1 task=1
```

- [ ] **Step 4: Update README**

Document that the supervisor flow now demonstrates a kernel-owned task lifecycle
instead of an externally invented `TaskId`.

- [ ] **Step 5: Run supervisor**

Run:

```bash
PATH="$HOME/.cargo/bin:$PATH" RUSTC="$(rustup which rustc --toolchain nightly)" rustup run nightly cargo run -p agent-supervisor
```

Expected: event log includes events 1 through 10 and the task lifecycle labels.

## Task 5: Verification And Publish

**Files:**
- Review all changed files.

- [ ] **Step 1: Format**

Run:

```bash
PATH="$HOME/.cargo/bin:$PATH" rustup run nightly cargo fmt --check
```

Expected: exit 0.

- [ ] **Step 2: Test workspace**

Run:

```bash
PATH="$HOME/.cargo/bin:$PATH" RUSTC="$(rustup which rustc --toolchain nightly)" rustup run nightly cargo test --workspace
```

Expected: exit 0.

- [ ] **Step 3: Run supervisor**

Run:

```bash
PATH="$HOME/.cargo/bin:$PATH" RUSTC="$(rustup which rustc --toolchain nightly)" rustup run nightly cargo run -p agent-supervisor
```

Expected: task lifecycle events print in order.

- [ ] **Step 4: Run QEMU**

Run:

```bash
scripts/run-qemu.sh
```

Expected: serial output reaches `SUPERVISOR_HANDOFF_READY`.

- [ ] **Step 5: Commit and push**

Run:

```bash
git status -sb
git diff --check
git add README.md crates/agent-kernel-core crates/agent-kernel crates/agent-kernel-boot crates/agent-kernel-x86_64 crates/agent-supervisor docs/superpowers/plans/2026-07-02-task-store-lifecycle-v0.md
git commit -m "feat: add task store lifecycle"
git push origin main
```

Expected: commit pushed to `origin/main`.
