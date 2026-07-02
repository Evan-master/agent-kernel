# Delegated Capability Derivation V0 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Derive task-scoped action capabilities during delegation so assignees can complete delegated tasks without broad resource grants.

**Architecture:** `agent-kernel-core` extends capabilities with optional task scope and stores the derived capability on each delegated task. Generic resource authorization rejects scoped capabilities, while task lifecycle completion accepts either normal resource authority or a matching task-scoped capability. `agent-kernel` keeps the same syscalls and exposes the derived capability through the existing read-only task view.

**Tech Stack:** Rust nightly, no_std core and facade crates, fixed-capacity arrays, Cargo workspace tests, QEMU BIOS boot script.

---

## File Structure

- Create `crates/agent-kernel-core/tests/delegated_capability.rs` for red tests around task-scoped derivation.
- Modify `crates/agent-kernel-core/src/capability.rs` to add `task: Option<TaskId>`.
- Modify `crates/agent-kernel-core/src/task.rs` to add `delegated_capability: Option<CapabilityId>`.
- Modify `crates/agent-kernel-core/src/error.rs` to add `CapabilityScopeMismatch`.
- Modify `crates/agent-kernel-core/src/capability_store.rs` to create normal grants with no task scope and provide internal task derivation.
- Modify `crates/agent-kernel-core/src/authorization.rs` to reject scoped capabilities for generic resource operations and add task-scoped authorization.
- Modify `crates/agent-kernel-core/src/task_store.rs` to derive capability during delegation and use task authorization during completion.
- Modify existing tests and supervisor to use derived task capabilities instead of manual assignee grants.
- Modify `README.md` to document task-scoped delegated capability behavior.

## Task 1: Core Delegated Capability Red Tests

**Files:**
- Create: `crates/agent-kernel-core/tests/delegated_capability.rs`

- [ ] **Step 1: Add delegated capability tests**

Create `crates/agent-kernel-core/tests/delegated_capability.rs`:

```rust
use agent_kernel_core::{
    ActionId, AgentId, CapabilityId, EventKind, KernelCore, KernelError, Operation, OperationSet,
    ResourceId, ResourceKind, TaskId, TaskStatus,
};

type TestCore = KernelCore<4, 8, 32, 6, 4>;

#[derive(Copy, Clone)]
struct DelegatedTask {
    task: TaskId,
    resource: ResourceId,
    owner_capability: CapabilityId,
    delegated_capability: CapabilityId,
}

fn create_delegated_task(core: &mut TestCore, owner: AgentId, assignee: AgentId) -> DelegatedTask {
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
                .with(Operation::Verify)
                .with(Operation::Rollback),
        )
        .expect("owner capability should fit");
    let task = core
        .create_task(owner, owner_capability, resource)
        .expect("task should be created");
    let event = core
        .delegate_task(owner, owner_capability, task, assignee)
        .expect("task should be delegated");
    let delegated_capability = event
        .capability
        .expect("delegation should expose derived capability");

    DelegatedTask {
        task,
        resource,
        owner_capability,
        delegated_capability,
    }
}

fn dispatch_task(core: &mut TestCore, assignee: AgentId, task: TaskId) {
    core.accept_task(assignee, task)
        .expect("task should be accepted");
    core.enqueue_task(assignee, task)
        .expect("task should enqueue");
    core.dispatch_next(assignee)
        .expect("task should dispatch");
}

#[test]
fn delegate_task_derives_task_scoped_capability_for_assignee() {
    let mut core = TestCore::new();
    let owner = AgentId::new(1);
    let assignee = AgentId::new(2);
    let delegated = create_delegated_task(&mut core, owner, assignee);

    assert_ne!(delegated.delegated_capability, delegated.owner_capability);
    assert_eq!(
        core.tasks()[0].delegated_capability,
        Some(delegated.delegated_capability)
    );
    assert_eq!(core.tasks()[0].assignee, Some(assignee));
    assert_eq!(core.tasks()[0].status, TaskStatus::Delegated);
    assert_eq!(core.events().last().unwrap().kind, EventKind::DelegationRequested);
    assert_eq!(
        core.events().last().unwrap().capability,
        Some(delegated.delegated_capability)
    );
}

#[test]
fn derived_capability_completes_dispatched_task_without_manual_grant() {
    let mut core = TestCore::new();
    let owner = AgentId::new(3);
    let assignee = AgentId::new(4);
    let delegated = create_delegated_task(&mut core, owner, assignee);
    dispatch_task(&mut core, assignee, delegated.task);

    let event = core
        .complete_task(assignee, delegated.delegated_capability, delegated.task)
        .expect("derived capability should complete assigned running task");

    assert_eq!(event.kind, EventKind::TaskCompleted);
    assert_eq!(core.tasks()[0].status, TaskStatus::Completed);
}

#[test]
fn derived_capability_cannot_authorize_generic_action() {
    let mut core = TestCore::new();
    let owner = AgentId::new(5);
    let assignee = AgentId::new(6);
    let delegated = create_delegated_task(&mut core, owner, assignee);
    let events_before = core.events().len();

    let result = core.act(
        assignee,
        delegated.delegated_capability,
        ActionId::new(7),
        delegated.resource,
    );

    assert_eq!(result, Err(KernelError::CapabilityScopeMismatch));
    assert_eq!(core.events().len(), events_before);
}

#[test]
fn derived_capability_cannot_complete_a_different_task() {
    let mut core = TestCore::new();
    let owner = AgentId::new(7);
    let assignee = AgentId::new(8);
    let first = create_delegated_task(&mut core, owner, assignee);
    let second = create_delegated_task(&mut core, owner, assignee);
    dispatch_task(&mut core, assignee, second.task);
    let events_before = core.events().len();

    let result = core.complete_task(assignee, first.delegated_capability, second.task);

    assert_eq!(result, Err(KernelError::CapabilityScopeMismatch));
    assert_eq!(core.tasks()[1].status, TaskStatus::Running);
    assert_eq!(core.events().len(), events_before);
}

#[test]
fn delegate_requires_source_act_authority_for_derived_capability() {
    let mut core = TestCore::new();
    let owner = AgentId::new(9);
    let assignee = AgentId::new(10);
    let resource = core
        .register_resource(ResourceKind::Workspace, None)
        .expect("resource should fit");
    let create_capability = core
        .grant_capability(owner, resource, OperationSet::only(Operation::Act))
        .expect("create capability should fit");
    let delegate_only_capability = core
        .grant_capability(owner, resource, OperationSet::only(Operation::Delegate))
        .expect("delegate capability should fit");
    let task = core
        .create_task(owner, create_capability, resource)
        .expect("task should be created");
    let events_after_create = core.events().len();

    let result = core.delegate_task(owner, delegate_only_capability, task, assignee);

    assert_eq!(result, Err(KernelError::OperationDenied));
    assert_eq!(core.tasks()[0].status, TaskStatus::Created);
    assert_eq!(core.tasks()[0].delegated_capability, None);
    assert_eq!(core.events().len(), events_after_create);
}

#[test]
fn delegate_returns_capability_store_full_without_state_changes() {
    let mut core = KernelCore::<2, 1, 8, 2, 2>::new();
    let owner = AgentId::new(11);
    let assignee = AgentId::new(12);
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

    assert_eq!(result, Err(KernelError::CapabilityStoreFull));
    assert_eq!(core.tasks()[0].status, TaskStatus::Created);
    assert_eq!(core.tasks()[0].assignee, None);
    assert_eq!(core.tasks()[0].delegated_capability, None);
    assert_eq!(core.events().len(), events_after_create);
}
```

- [ ] **Step 2: Run red core tests**

Run:

```bash
PATH="$HOME/.cargo/bin:$PATH" RUSTC="$(rustup which rustc --toolchain nightly)" rustup run nightly cargo test -p agent-kernel-core --test delegated_capability
```

Expected: compile failures for missing `Task::delegated_capability`, `Capability::task`, and `KernelError::CapabilityScopeMismatch`.

## Task 2: Core Capability Scope Implementation

**Files:**
- Modify: `crates/agent-kernel-core/src/capability.rs`
- Modify: `crates/agent-kernel-core/src/task.rs`
- Modify: `crates/agent-kernel-core/src/error.rs`
- Modify: `crates/agent-kernel-core/src/capability_store.rs`
- Modify: `crates/agent-kernel-core/src/authorization.rs`
- Modify: `crates/agent-kernel-core/src/task_store.rs`

- [ ] **Step 1: Add task scope fields**

In `crates/agent-kernel-core/src/capability.rs`, import `TaskId` and add `task`:

```rust
use crate::{AgentId, CapabilityId, OperationSet, ResourceId, TaskId};

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Capability {
    pub id: CapabilityId,
    pub agent: AgentId,
    pub resource: ResourceId,
    pub operations: OperationSet,
    pub revoked: bool,
    pub task: Option<TaskId>,
}
```

In `crates/agent-kernel-core/src/task.rs`, import `CapabilityId` and add `delegated_capability`:

```rust
use crate::{AgentId, CapabilityId, ResourceId, TaskId};
```

Add the field:

```rust
pub delegated_capability: Option<CapabilityId>,
```

Set it to `None` in `Task::empty()`.

- [ ] **Step 2: Add scope error**

In `crates/agent-kernel-core/src/error.rs`, add:

```rust
CapabilityScopeMismatch,
```

- [ ] **Step 3: Add derived capability allocation**

In `crates/agent-kernel-core/src/capability_store.rs`, import `TaskId` and update normal grants:

```rust
task: None,
```

Add this method inside the existing `impl KernelCore` block:

```rust
pub(crate) fn derive_task_capability(
    &mut self,
    agent: AgentId,
    resource: ResourceId,
    operations: OperationSet,
    task: TaskId,
) -> Result<CapabilityId, KernelError> {
    self.find_resource(resource)?;

    let slot = self
        .capabilities
        .iter_mut()
        .find(|capability| capability.is_none())
        .ok_or(KernelError::CapabilityStoreFull)?;
    let id = CapabilityId::new(self.next_capability);
    self.next_capability += 1;
    *slot = Some(Capability {
        id,
        agent,
        resource,
        operations,
        revoked: false,
        task: Some(task),
    });
    Ok(id)
}
```

- [ ] **Step 4: Split authorization base checks and task-scope checks**

In `crates/agent-kernel-core/src/authorization.rs`, replace the implementation body with:

```rust
use crate::{
    AgentId, Capability, CapabilityId, KernelCore, KernelError, Operation, ResourceId, TaskId,
};

impl<
        const RESOURCES: usize,
        const CAPS: usize,
        const EVENTS: usize,
        const TASKS: usize,
        const RUN_QUEUE: usize,
    > KernelCore<RESOURCES, CAPS, EVENTS, TASKS, RUN_QUEUE>
{
    pub(crate) fn ensure_authorized(
        &self,
        agent: AgentId,
        capability: CapabilityId,
        resource: ResourceId,
        operation: Operation,
    ) -> Result<(), KernelError> {
        let cap = self.ensure_capability_base(agent, capability, resource, operation)?;
        if cap.task.is_some() {
            return Err(KernelError::CapabilityScopeMismatch);
        }

        Ok(())
    }

    pub(crate) fn ensure_authorized_for_task(
        &self,
        agent: AgentId,
        capability: CapabilityId,
        resource: ResourceId,
        operation: Operation,
        task: TaskId,
    ) -> Result<(), KernelError> {
        let cap = self.ensure_capability_base(agent, capability, resource, operation)?;
        if let Some(scope) = cap.task {
            if scope != task {
                return Err(KernelError::CapabilityScopeMismatch);
            }
        }

        Ok(())
    }

    fn ensure_capability_base(
        &self,
        agent: AgentId,
        capability: CapabilityId,
        resource: ResourceId,
        operation: Operation,
    ) -> Result<Capability, KernelError> {
        self.find_resource(resource)?;
        let cap = self.find_capability(capability)?;

        if cap.revoked {
            return Err(KernelError::CapabilityRevoked);
        }
        if cap.agent != agent {
            return Err(KernelError::AgentMismatch);
        }
        if cap.resource != resource {
            return Err(KernelError::ResourceMismatch);
        }
        if !cap.operations.allows(operation) {
            return Err(KernelError::OperationDenied);
        }

        Ok(cap)
    }
}
```

- [ ] **Step 5: Derive during delegation and authorize completion for task scope**

In `crates/agent-kernel-core/src/task_store.rs`, in `create_task`, initialize:

```rust
delegated_capability: None,
```

In `delegate_task`, after the existing delegate authorization, add source action validation:

```rust
self.ensure_authorized(agent, capability, current.resource, Operation::Act)?;
```

Before mutating the task, derive:

```rust
let delegated_capability = self.derive_task_capability(
    target_agent,
    current.resource,
    OperationSet::only(Operation::Act),
    task,
)?;
```

Then set:

```rust
task_ref.delegated_capability = Some(delegated_capability);
```

Record delegation with the derived capability:

```rust
Some(delegated_capability),
```

In `complete_task`, replace:

```rust
self.ensure_authorized(agent, capability, current.resource, Operation::Act)?;
```

with:

```rust
self.ensure_authorized_for_task(agent, capability, current.resource, Operation::Act, task)?;
```

- [ ] **Step 6: Run core delegated capability tests**

Run:

```bash
PATH="$HOME/.cargo/bin:$PATH" RUSTC="$(rustup which rustc --toolchain nightly)" rustup run nightly cargo test -p agent-kernel-core --test delegated_capability
```

Expected: delegated capability tests pass.

## Task 3: Update Existing Tests For Derived Capabilities

**Files:**
- Modify: `crates/agent-kernel-core/tests/task_lifecycle.rs`
- Modify: `crates/agent-kernel-core/tests/task_authority.rs`
- Modify: `crates/agent-kernel-core/tests/scheduler.rs`
- Modify: `crates/agent-kernel/tests/kernel_facade.rs`

- [ ] **Step 1: Update task assertions for new field**

In `crates/agent-kernel-core/tests/task_lifecycle.rs`, add to `create_task_allocates_kernel_task_and_records_event`:

```rust
assert_eq!(core.tasks()[0].delegated_capability, None);
```

- [ ] **Step 2: Use derived capability in successful core lifecycle tests**

Where tests currently grant `assignee_capability` only to complete delegated work, remove that grant and after delegation read:

```rust
let assignee_capability = core.tasks()[0]
    .delegated_capability
    .expect("delegation should derive assignee capability");
```

Apply this in:

- `crates/agent-kernel-core/tests/task_lifecycle.rs`
- `crates/agent-kernel-core/tests/task_authority.rs`
- `crates/agent-kernel-core/tests/scheduler.rs`

Keep explicit owner capabilities for create, delegate, verify, and cancel.

- [ ] **Step 3: Update facade lifecycle tests**

In `crates/agent-kernel/tests/kernel_facade.rs`, remove manual assignee resource grants from the full lifecycle and rejection tests. After `sys_delegate_task`, read:

```rust
let assignee_capability = kernel.tasks()[0]
    .delegated_capability
    .expect("delegation should derive assignee capability");
```

Use this derived capability for `sys_complete_task`.

- [ ] **Step 4: Run core and facade tests**

Run:

```bash
PATH="$HOME/.cargo/bin:$PATH" RUSTC="$(rustup which rustc --toolchain nightly)" rustup run nightly cargo test -p agent-kernel-core
PATH="$HOME/.cargo/bin:$PATH" RUSTC="$(rustup which rustc --toolchain nightly)" rustup run nightly cargo test -p agent-kernel --test kernel_facade
```

Expected: all core and facade tests pass.

## Task 4: Supervisor And README

**Files:**
- Modify: `crates/agent-supervisor/src/main.rs`
- Modify: `README.md`

- [ ] **Step 1: Remove manual assignee grant in supervisor**

In `crates/agent-supervisor/src/main.rs`, remove:

```rust
let assignee_capability = kernel
    .sys_grant(target_agent, workspace, OperationSet::only(Operation::Act))
    .expect("target agent capability should fit in simulator kernel");
```

After `sys_delegate_task`, add:

```rust
let assignee_capability = kernel.tasks()[0]
    .delegated_capability
    .expect("delegation should derive target agent capability");
```

Use that capability for `sys_complete_task`.

- [ ] **Step 2: Update README**

Add one sentence after the paragraph describing the run queue:

```markdown
Delegation derives a task-scoped action capability for the assignee, so the supervisor does not grant broad resource authority to complete delegated work.
```

- [ ] **Step 3: Run supervisor**

Run:

```bash
PATH="$HOME/.cargo/bin:$PATH" RUSTC="$(rustup which rustc --toolchain nightly)" rustup run nightly cargo run -p agent-supervisor
```

Expected: output still includes task accepted, queued, dispatched, completed, and verified events in order.

## Task 5: Workspace Verification And Publish

**Files:**
- Review all changed files.

- [ ] **Step 1: Format**

Run:

```bash
PATH="$HOME/.cargo/bin:$PATH" rustup run nightly cargo fmt
PATH="$HOME/.cargo/bin:$PATH" rustup run nightly cargo fmt --check
```

Expected: exit 0.

- [ ] **Step 2: Workspace tests**

Run:

```bash
PATH="$HOME/.cargo/bin:$PATH" RUSTC="$(rustup which rustc --toolchain nightly)" rustup run nightly cargo test --workspace
```

Expected: all tests pass.

- [ ] **Step 3: Supervisor run**

Run:

```bash
PATH="$HOME/.cargo/bin:$PATH" RUSTC="$(rustup which rustc --toolchain nightly)" rustup run nightly cargo run -p agent-supervisor
```

Expected: output still includes:

```text
event[8] task_accepted agent=2 resource=1 task=1
event[9] task_queued agent=2 resource=1 task=1
event[10] task_dispatched agent=2 resource=1 task=1
event[11] task_completed agent=2 resource=1 task=1
event[12] task_verified agent=1 resource=1 task=1
```

- [ ] **Step 4: QEMU boot**

Run:

```bash
scripts/run-qemu.sh
```

Expected: serial output reaches `SUPERVISOR_HANDOFF_READY`.

- [ ] **Step 5: Scope and line-size check**

Run:

```bash
wc -l crates/agent-kernel-core/src/*.rs crates/agent-kernel-core/tests/*.rs crates/agent-kernel/src/*.rs crates/agent-kernel/tests/*.rs crates/agent-supervisor/src/main.rs README.md
git diff --check
git status -sb
```

Expected: no hard-limit file sizes, no whitespace errors, only intended delegated-capability files changed.

- [ ] **Step 6: Commit and push**

Run:

```bash
git add README.md crates docs/superpowers/plans/2026-07-02-delegated-capability-derivation-v0.md
git commit -m "feat: derive delegated task capabilities"
git push origin main
```

Expected: commit pushed to `origin/main`.
