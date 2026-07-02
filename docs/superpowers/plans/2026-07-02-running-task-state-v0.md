# Running Task State V0 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add `TaskStatus::Running` so dispatched tasks become running and tasks cannot be completed before dispatch.

**Architecture:** `agent-kernel-core` owns the lifecycle change: dispatch mutates task status, yield moves running work back to accepted, and completion requires running. `agent-kernel` keeps the same syscall names while exposing stronger core semantics. `agent-supervisor` keeps the same event flow but now relies on dispatch as a real task-state transition.

**Tech Stack:** Rust nightly, no_std core and facade crates, fixed-capacity arrays, Cargo workspace tests, QEMU BIOS boot script.

---

## File Structure

- Modify `crates/agent-kernel-core/src/task.rs` to add `TaskStatus::Running`.
- Modify `crates/agent-kernel-core/src/scheduler.rs` to set `Running` during dispatch and require `Running` for yield.
- Modify `crates/agent-kernel-core/src/task_store.rs` to require `Running` for completion and allow cancellation while running.
- Modify `crates/agent-kernel-core/tests/scheduler.rs` for dispatch/yield/running-state tests.
- Modify `crates/agent-kernel-core/tests/task_lifecycle.rs` and `task_authority.rs` so full lifecycle tests dispatch before completion.
- Modify `crates/agent-kernel/tests/kernel_facade.rs` so facade lifecycle and yield tests follow the running-state contract.
- Modify `README.md` to document that dispatch changes task status to running.

## Task 1: Core Running-State Red Tests

**Files:**
- Modify: `crates/agent-kernel-core/tests/scheduler.rs`

- [ ] **Step 1: Expand scheduler test imports and helper**

Change the import block to:

```rust
use agent_kernel_core::{
    AgentId, CapabilityId, EventKind, KernelCore, KernelError, Operation, OperationSet,
    ResourceKind, RunQueueEntry, TaskId, TaskStatus,
};
```

Add this helper below `type TestCore = KernelCore<4, 6, 32, 6, 4>;`:

```rust
#[derive(Copy, Clone)]
struct AcceptedTask {
    task: TaskId,
    owner_capability: CapabilityId,
    assignee_capability: CapabilityId,
}
```

Replace the existing `accepted_task` helper with:

```rust
fn accepted_task<
    const RESOURCES: usize,
    const CAPS: usize,
    const EVENTS: usize,
    const TASKS: usize,
    const RUN_QUEUE: usize,
>(
    core: &mut KernelCore<RESOURCES, CAPS, EVENTS, TASKS, RUN_QUEUE>,
    owner: AgentId,
    assignee: AgentId,
) -> TaskId {
    accepted_task_with_capabilities(core, owner, assignee).task
}

fn accepted_task_with_capabilities<
    const RESOURCES: usize,
    const CAPS: usize,
    const EVENTS: usize,
    const TASKS: usize,
    const RUN_QUEUE: usize,
>(
    core: &mut KernelCore<RESOURCES, CAPS, EVENTS, TASKS, RUN_QUEUE>,
    owner: AgentId,
    assignee: AgentId,
) -> AcceptedTask {
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
    let assignee_capability = core
        .grant_capability(assignee, resource, OperationSet::only(Operation::Act))
        .expect("assignee capability should fit");
    let task = core
        .create_task(owner, owner_capability, resource)
        .expect("task should be created");
    core.delegate_task(owner, owner_capability, task, assignee)
        .expect("task should be delegated");
    core.accept_task(assignee, task)
        .expect("task should be accepted");
    AcceptedTask {
        task,
        owner_capability,
        assignee_capability,
    }
}
```

- [ ] **Step 2: Update dispatch test to expect Running**

In `dispatch_next_pops_oldest_task_and_records_event`, add this assertion after `assert_eq!(dispatched, first);`:

```rust
assert_eq!(core.tasks()[0].status, TaskStatus::Running);
```

- [ ] **Step 3: Replace yield test with running-only yield behavior**

Replace `yield_task_requeues_accepted_task_at_back` with:

```rust
#[test]
fn yield_task_requeues_running_task_as_accepted_at_back() {
    let mut core = TestCore::new();
    let owner = AgentId::new(13);
    let first_agent = AgentId::new(14);
    let second_agent = AgentId::new(15);
    let first = accepted_task(&mut core, owner, first_agent);
    let second = accepted_task(&mut core, owner, second_agent);
    core.enqueue_task(first_agent, first)
        .expect("first task should enqueue");
    core.enqueue_task(second_agent, second)
        .expect("second task should enqueue");
    core.dispatch_next(first_agent)
        .expect("first task should dispatch");

    let event = core
        .yield_task(first_agent, first)
        .expect("running task should yield into queue");

    assert_eq!(event.kind, EventKind::TaskYielded);
    assert_eq!(core.tasks()[0].status, TaskStatus::Accepted);
    assert_eq!(
        core.run_queue(),
        &[
            RunQueueEntry {
                task: second,
                agent: second_agent,
            },
            RunQueueEntry {
                task: first,
                agent: first_agent,
            },
        ]
    );
}
```

- [ ] **Step 4: Add completion and cancellation tests**

Append these tests to `crates/agent-kernel-core/tests/scheduler.rs`:

```rust
#[test]
fn completing_accepted_task_before_dispatch_is_rejected_without_events() {
    let mut core = TestCore::new();
    let owner = AgentId::new(16);
    let assignee = AgentId::new(17);
    let accepted = accepted_task_with_capabilities(&mut core, owner, assignee);
    let events_before = core.events().len();

    let result = core.complete_task(assignee, accepted.assignee_capability, accepted.task);

    assert_eq!(result, Err(KernelError::TaskStatusMismatch));
    assert_eq!(core.tasks()[0].status, TaskStatus::Accepted);
    assert_eq!(core.events().len(), events_before);
}

#[test]
fn completing_running_task_records_completed_status() {
    let mut core = TestCore::new();
    let owner = AgentId::new(18);
    let assignee = AgentId::new(19);
    let accepted = accepted_task_with_capabilities(&mut core, owner, assignee);
    core.enqueue_task(assignee, accepted.task)
        .expect("accepted task should enqueue");
    core.dispatch_next(assignee)
        .expect("accepted task should dispatch");

    let event = core
        .complete_task(assignee, accepted.assignee_capability, accepted.task)
        .expect("running task should complete");

    assert_eq!(event.kind, EventKind::TaskCompleted);
    assert_eq!(core.tasks()[0].status, TaskStatus::Completed);
}

#[test]
fn yielding_accepted_task_without_dispatch_is_rejected_without_state_changes() {
    let mut core = TestCore::new();
    let owner = AgentId::new(20);
    let assignee = AgentId::new(21);
    let accepted = accepted_task_with_capabilities(&mut core, owner, assignee);
    let events_before = core.events().len();

    let result = core.yield_task(assignee, accepted.task);

    assert_eq!(result, Err(KernelError::TaskNotRunnable));
    assert_eq!(core.tasks()[0].status, TaskStatus::Accepted);
    assert!(core.run_queue().is_empty());
    assert_eq!(core.events().len(), events_before);
}

#[test]
fn cancelling_running_task_marks_cancelled_and_blocks_completion() {
    let mut core = TestCore::new();
    let owner = AgentId::new(22);
    let assignee = AgentId::new(23);
    let accepted = accepted_task_with_capabilities(&mut core, owner, assignee);
    core.enqueue_task(assignee, accepted.task)
        .expect("accepted task should enqueue");
    core.dispatch_next(assignee)
        .expect("accepted task should dispatch");

    let event = core
        .cancel_task(owner, accepted.owner_capability, accepted.task)
        .expect("running task should cancel");

    assert_eq!(event.kind, EventKind::TaskCancelled);
    assert_eq!(core.tasks()[0].status, TaskStatus::Cancelled);
    assert_eq!(
        core.complete_task(assignee, accepted.assignee_capability, accepted.task),
        Err(KernelError::TaskStatusMismatch)
    );
}
```

- [ ] **Step 5: Run red core scheduler tests**

Run:

```bash
PATH="$HOME/.cargo/bin:$PATH" RUSTC="$(rustup which rustc --toolchain nightly)" rustup run nightly cargo test -p agent-kernel-core --test scheduler
```

Expected: compile failure for missing `TaskStatus::Running`, and after that lifecycle failures showing dispatch does not set running and yield does not require running.

## Task 2: Core Running-State Implementation

**Files:**
- Modify: `crates/agent-kernel-core/src/task.rs`
- Modify: `crates/agent-kernel-core/src/scheduler.rs`
- Modify: `crates/agent-kernel-core/src/task_store.rs`
- Modify: `crates/agent-kernel-core/tests/task_lifecycle.rs`
- Modify: `crates/agent-kernel-core/tests/task_authority.rs`

- [ ] **Step 1: Add `Running` status**

In `crates/agent-kernel-core/src/task.rs`, change `TaskStatus` to:

```rust
pub enum TaskStatus {
    Created,
    Delegated,
    Accepted,
    Running,
    Completed,
    Verified,
    Cancelled,
}
```

- [ ] **Step 2: Update scheduler status transitions**

In `crates/agent-kernel-core/src/scheduler.rs`, change `dispatch_next` so after event-capacity validation and before recording the event it mutates the task:

```rust
self.shift_run_queue_left();
self.find_task_mut(entry.task)?.status = TaskStatus::Running;
self.record_scheduler_event(
    EventKind::TaskDispatched,
    agent,
    entry.task,
    task_record.resource,
)?;
Ok(entry.task)
```

Replace `yield_task` with:

```rust
pub fn yield_task(&mut self, agent: AgentId, task: TaskId) -> Result<Event, KernelError> {
    let task_record = self.find_task(task)?;
    if task_record.status != TaskStatus::Running || task_record.assignee != Some(agent) {
        return Err(KernelError::TaskNotRunnable);
    }
    self.ensure_not_queued(task)?;
    self.ensure_run_queue_capacity()?;
    self.ensure_scheduler_event_capacity()?;

    self.find_task_mut(task)?.status = TaskStatus::Accepted;
    self.run_queue[self.run_queue_len] = RunQueueEntry { task, agent };
    self.run_queue_len += 1;
    self.record_scheduler_event(EventKind::TaskYielded, agent, task, task_record.resource)
}
```

Keep `find_runnable_task` accepted-only so enqueue and dispatch queue validation still reject `Running` tasks.

- [ ] **Step 3: Update task lifecycle status guards**

In `crates/agent-kernel-core/src/task_store.rs`, change `complete_task` status validation to:

```rust
ensure_status(current.status, &[TaskStatus::Running])?;
```

In `cancel_task`, include `TaskStatus::Running` in the allowed status list:

```rust
ensure_status(
    current.status,
    &[
        TaskStatus::Created,
        TaskStatus::Delegated,
        TaskStatus::Accepted,
        TaskStatus::Running,
        TaskStatus::Completed,
    ],
)?;
```

- [ ] **Step 4: Update existing core lifecycle tests to dispatch before complete**

In `crates/agent-kernel-core/tests/task_lifecycle.rs`, add this after `accept_task` in `task_lifecycle_reaches_verified_through_authorized_transitions`:

```rust
core.enqueue_task(assignee, task)
    .expect("task should enqueue");
core.dispatch_next(assignee)
    .expect("task should dispatch");
```

Update the expected event sequence:

```rust
assert_eq!(core.events()[3].kind, EventKind::TaskQueued);
assert_eq!(core.events()[4].kind, EventKind::TaskDispatched);
assert_eq!(core.events()[5].kind, EventKind::TaskCompleted);
assert_eq!(core.events()[6].kind, EventKind::TaskVerified);
```

In `crates/agent-kernel-core/tests/task_authority.rs`, add the same enqueue/dispatch pair before each successful `complete_task` call in `verified_task_rejects_further_transitions_without_events`.

- [ ] **Step 5: Run core tests**

Run:

```bash
PATH="$HOME/.cargo/bin:$PATH" RUSTC="$(rustup which rustc --toolchain nightly)" rustup run nightly cargo test -p agent-kernel-core
```

Expected: all core tests pass.

## Task 3: Facade Tests And Documentation Updates

**Files:**
- Modify: `crates/agent-kernel/tests/kernel_facade.rs`
- Modify: `README.md`

- [ ] **Step 1: Update full facade lifecycle to dispatch before complete**

In `task_syscalls_record_full_task_lifecycle`, add after `sys_accept_task`:

```rust
kernel
    .sys_enqueue_task(assignee, task)
    .expect("task should enqueue");
kernel
    .sys_dispatch_next(assignee)
    .expect("task should dispatch");
```

Update event assertions:

```rust
assert_eq!(kernel.events()[3].kind, EventKind::TaskQueued);
assert_eq!(kernel.events()[4].kind, EventKind::TaskDispatched);
assert_eq!(kernel.events()[5].kind, EventKind::TaskCompleted);
assert_eq!(kernel.events()[6].kind, EventKind::TaskVerified);
```

- [ ] **Step 2: Update facade scheduler yield test**

In `scheduler_syscalls_enqueue_dispatch_and_yield_tasks`, replace the queue/yield/dispatch block with:

```rust
kernel
    .sys_enqueue_task(first_agent, first)
    .expect("first task should enqueue");
kernel
    .sys_enqueue_task(second_agent, second)
    .expect("second task should enqueue");
let dispatched = kernel
    .sys_dispatch_next(first_agent)
    .expect("first task should dispatch");
kernel
    .sys_yield_task(first_agent, first)
    .expect("running task should yield into queue");

assert_eq!(dispatched, first);
assert_eq!(kernel.tasks()[0].status, TaskStatus::Accepted);
```

Keep the final queue assertion so the queue contains `second` followed by `first`.

- [ ] **Step 3: Add facade rejection test**

Append:

```rust
#[test]
fn completing_task_before_dispatch_is_rejected_by_facade() {
    let mut kernel = TestKernel::new();
    let owner = AgentId::new(203);
    let assignee = AgentId::new(204);
    let resource = kernel
        .sys_register_resource(ResourceKind::Workspace, None)
        .expect("resource should fit");
    let owner_capability = kernel
        .sys_grant(
            owner,
            resource,
            OperationSet::empty()
                .with(Operation::Act)
                .with(Operation::Delegate),
        )
        .expect("owner capability should fit");
    let assignee_capability = kernel
        .sys_grant(assignee, resource, OperationSet::only(Operation::Act))
        .expect("assignee capability should fit");
    let task = kernel
        .sys_create_task(owner, owner_capability, resource)
        .expect("task should be created");
    kernel
        .sys_delegate_task(owner, owner_capability, task, assignee)
        .expect("task should delegate");
    kernel
        .sys_accept_task(assignee, task)
        .expect("task should accept");
    let events_before = kernel.events().len();

    let result = kernel.sys_complete_task(assignee, assignee_capability, task);

    assert_eq!(result, Err(agent_kernel_core::KernelError::TaskStatusMismatch));
    assert_eq!(kernel.tasks()[0].status, TaskStatus::Accepted);
    assert_eq!(kernel.events().len(), events_before);
}
```

- [ ] **Step 4: Update README wording**

Change the current behavior line:

```markdown
11. Enqueue and dispatch the accepted task through the kernel run queue.
12. Let the assignee complete the dispatched task.
```

to:

```markdown
11. Enqueue the accepted task and dispatch it into `Running` state through the kernel run queue.
12. Let the assignee complete the running task.
```

Change the paragraph:

```markdown
Accepted tasks move through a fixed-capacity FIFO run queue before completion.
```

to:

```markdown
Accepted tasks move through a fixed-capacity FIFO run queue and become `Running` before completion.
```

- [ ] **Step 5: Run facade tests**

Run:

```bash
PATH="$HOME/.cargo/bin:$PATH" RUSTC="$(rustup which rustc --toolchain nightly)" rustup run nightly cargo test -p agent-kernel --test kernel_facade
```

Expected: all facade tests pass.

## Task 4: Workspace Verification And Publish

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

Expected: no hard-limit file sizes, no whitespace errors, only intended running-state files changed.

- [ ] **Step 6: Commit and push**

Run:

```bash
git add README.md crates docs/superpowers/plans/2026-07-02-running-task-state-v0.md
git commit -m "feat: add running task state"
git push origin main
```

Expected: commit pushed to `origin/main`.
