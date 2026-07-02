# Scheduler Run Queue V0 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a deterministic FIFO run queue so the kernel can enqueue accepted tasks, dispatch the next runnable task, and record scheduler events.

**Architecture:** `agent-kernel-core` owns fixed-capacity run queue state and scheduler transition validation. `agent-kernel` exposes syscall-style wrappers without exposing mutable core state. `agent-supervisor` demonstrates the queue through facade calls only.

**Tech Stack:** Rust nightly, no_std core and facade crates, fixed-capacity arrays, existing Cargo workspace, QEMU BIOS boot script.

---

## File Structure

- Create `crates/agent-kernel-core/src/run_queue.rs` for `RunQueueEntry`.
- Create `crates/agent-kernel-core/src/scheduler.rs` for enqueue, dispatch, yield, queue inspection, and scheduler event recording.
- Modify `crates/agent-kernel-core/src/core.rs` to add `RUN_QUEUE`, queue storage, and queue length.
- Modify `crates/agent-kernel-core/src/error.rs` to add run queue errors.
- Modify `crates/agent-kernel-core/src/event.rs` to add scheduler event kinds.
- Modify `crates/agent-kernel-core/src/lib.rs` to export `RunQueueEntry` and include scheduler modules.
- Modify every `KernelCore<...>` impl helper module to include `RUN_QUEUE`.
- Modify `crates/agent-kernel/src/lib.rs` to mirror `RUN_QUEUE` and expose scheduler syscalls.
- Modify `crates/agent-kernel-boot`, `crates/agent-kernel-x86_64`, and tests to use five generic parameters.
- Modify `crates/agent-supervisor` and `README.md` to demonstrate and document queue events.

## Task 1: Core Scheduler Red Tests

**Files:**
- Create: `crates/agent-kernel-core/tests/scheduler.rs`

- [ ] **Step 1: Write scheduler tests**

Create `crates/agent-kernel-core/tests/scheduler.rs`:

```rust
use agent_kernel_core::{
    AgentId, EventKind, KernelCore, KernelError, Operation, OperationSet, ResourceKind,
    RunQueueEntry, TaskId, TaskStatus,
};

type TestCore = KernelCore<4, 6, 32, 6, 4>;

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
    let task = core
        .create_task(owner, owner_capability, resource)
        .expect("task should be created");
    core.delegate_task(owner, owner_capability, task, assignee)
        .expect("task should be delegated");
    core.accept_task(assignee, task)
        .expect("task should be accepted");
    task
}

#[test]
fn enqueue_accepted_task_records_fifo_entry() {
    let mut core = TestCore::new();
    let owner = AgentId::new(1);
    let assignee = AgentId::new(2);
    let task = accepted_task(&mut core, owner, assignee);
    let events_before_enqueue = core.events().len();

    let event = core
        .enqueue_task(assignee, task)
        .expect("accepted task should enqueue");

    assert_eq!(event.kind, EventKind::TaskQueued);
    assert_eq!(event.task, Some(task));
    assert_eq!(event.agent, assignee);
    assert_eq!(core.run_queue(), &[RunQueueEntry { task, agent: assignee }]);
    assert_eq!(core.events().len(), events_before_enqueue + 1);
}

#[test]
fn dispatch_next_pops_oldest_task_and_records_event() {
    let mut core = TestCore::new();
    let owner = AgentId::new(3);
    let first_agent = AgentId::new(4);
    let second_agent = AgentId::new(5);
    let first = accepted_task(&mut core, owner, first_agent);
    let second = accepted_task(&mut core, owner, second_agent);
    core.enqueue_task(first_agent, first)
        .expect("first task should enqueue");
    core.enqueue_task(second_agent, second)
        .expect("second task should enqueue");

    let dispatched = core
        .dispatch_next(first_agent)
        .expect("first agent should dispatch first queued task");

    assert_eq!(dispatched, first);
    assert_eq!(
        core.run_queue(),
        &[RunQueueEntry {
            task: second,
            agent: second_agent
        }]
    );
    let last = core.events().last().expect("dispatch should record event");
    assert_eq!(last.kind, EventKind::TaskDispatched);
    assert_eq!(last.task, Some(first));
    assert_eq!(last.agent, first_agent);
}

#[test]
fn scheduler_rejects_invalid_queue_operations_without_state_changes() {
    let mut core = TestCore::new();
    let owner = AgentId::new(6);
    let assignee = AgentId::new(7);
    let wrong_agent = AgentId::new(8);
    let resource = core
        .register_resource(ResourceKind::Workspace, None)
        .expect("resource should fit");
    let capability = core
        .grant_capability(owner, resource, OperationSet::only(Operation::Act))
        .expect("capability should fit");
    let created = core
        .create_task(owner, capability, resource)
        .expect("task should be created");
    let accepted = accepted_task(&mut core, owner, assignee);
    core.enqueue_task(assignee, accepted)
        .expect("accepted task should enqueue");
    let queue_before = *core.run_queue().first().expect("queue should have entry");
    let events_before = core.events().len();

    assert_eq!(
        core.enqueue_task(owner, created),
        Err(KernelError::TaskNotRunnable)
    );
    assert_eq!(
        core.enqueue_task(wrong_agent, accepted),
        Err(KernelError::TaskNotRunnable)
    );
    assert_eq!(
        core.enqueue_task(assignee, accepted),
        Err(KernelError::TaskAlreadyQueued)
    );
    assert_eq!(
        core.dispatch_next(wrong_agent),
        Err(KernelError::TaskNotRunnable)
    );
    assert_eq!(core.run_queue(), &[queue_before]);
    assert_eq!(core.events().len(), events_before);
}

#[test]
fn enqueue_returns_run_queue_full_when_capacity_is_exhausted() {
    let mut core = KernelCore::<4, 6, 32, 4, 1>::new();
    let owner = AgentId::new(9);
    let first_agent = AgentId::new(10);
    let second_agent = AgentId::new(11);
    let first = accepted_task(&mut core, owner, first_agent);
    let second = accepted_task(&mut core, owner, second_agent);

    core.enqueue_task(first_agent, first)
        .expect("first task should enqueue");
    let result = core.enqueue_task(second_agent, second);

    assert_eq!(result, Err(KernelError::RunQueueFull));
    assert_eq!(
        core.run_queue(),
        &[RunQueueEntry {
            task: first,
            agent: first_agent
        }]
    );
}

#[test]
fn dispatch_from_empty_queue_returns_run_queue_empty() {
    let mut core = TestCore::new();

    let result = core.dispatch_next(AgentId::new(12));

    assert_eq!(result, Err(KernelError::RunQueueEmpty));
    assert!(core.run_queue().is_empty());
    assert!(core.events().is_empty());
}

#[test]
fn yield_task_requeues_accepted_task_at_back() {
    let mut core = TestCore::new();
    let owner = AgentId::new(13);
    let first_agent = AgentId::new(14);
    let second_agent = AgentId::new(15);
    let first = accepted_task(&mut core, owner, first_agent);
    let second = accepted_task(&mut core, owner, second_agent);
    core.enqueue_task(second_agent, second)
        .expect("second task should enqueue first");

    let event = core
        .yield_task(first_agent, first)
        .expect("accepted task should yield into queue");

    assert_eq!(event.kind, EventKind::TaskYielded);
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

- [ ] **Step 2: Run red tests**

Run:

```bash
PATH="$HOME/.cargo/bin:$PATH" RUSTC="$(rustup which rustc --toolchain nightly)" rustup run nightly cargo test -p agent-kernel-core scheduler
```

Expected: compile failures for missing `RunQueueEntry`, `TaskQueued`, `TaskDispatched`, `TaskYielded`, run queue errors, fifth `KernelCore` generic, and scheduler methods.

## Task 2: Core Run Queue Model And Scheduler

**Files:**
- Create: `crates/agent-kernel-core/src/run_queue.rs`
- Create: `crates/agent-kernel-core/src/scheduler.rs`
- Modify: `crates/agent-kernel-core/src/core.rs`
- Modify: `crates/agent-kernel-core/src/error.rs`
- Modify: `crates/agent-kernel-core/src/event.rs`
- Modify: `crates/agent-kernel-core/src/lib.rs`
- Modify: `crates/agent-kernel-core/src/authorization.rs`
- Modify: `crates/agent-kernel-core/src/capability_store.rs`
- Modify: `crates/agent-kernel-core/src/event_log.rs`
- Modify: `crates/agent-kernel-core/src/lookup.rs`
- Modify: `crates/agent-kernel-core/src/resource_store.rs`
- Modify: `crates/agent-kernel-core/src/task_store.rs`
- Modify: existing core tests to add fifth generic parameter.

- [ ] **Step 1: Add run queue entry model**

Create `crates/agent-kernel-core/src/run_queue.rs`:

```rust
//! Kernel-owned run queue entry model.
//!
//! This module belongs to `agent-kernel-core`. It defines the compact copyable
//! entry used by the fixed-capacity FIFO scheduler. It has no allocation or
//! host dependencies.

use crate::{AgentId, TaskId};

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct RunQueueEntry {
    pub task: TaskId,
    pub agent: AgentId,
}

impl RunQueueEntry {
    pub(crate) const fn empty() -> Self {
        Self {
            task: TaskId::new(0),
            agent: AgentId::new(0),
        }
    }
}
```

- [ ] **Step 2: Add errors and event kinds**

In `crates/agent-kernel-core/src/error.rs`, add:

```rust
RunQueueFull,
RunQueueEmpty,
TaskNotRunnable,
TaskAlreadyQueued,
```

In `crates/agent-kernel-core/src/event.rs`, add:

```rust
TaskQueued,
TaskDispatched,
TaskYielded,
```

- [ ] **Step 3: Add run queue storage to `KernelCore`**

Change `KernelCore` to:

```rust
pub struct KernelCore<
    const RESOURCES: usize,
    const CAPS: usize,
    const EVENTS: usize,
    const TASKS: usize,
    const RUN_QUEUE: usize,
> {
    pub(crate) resources: [Option<Resource>; RESOURCES],
    pub(crate) capabilities: [Option<Capability>; CAPS],
    pub(crate) events: [Event; EVENTS],
    pub(crate) tasks: [Task; TASKS],
    pub(crate) run_queue: [RunQueueEntry; RUN_QUEUE],
    pub(crate) event_len: usize,
    pub(crate) task_len: usize,
    pub(crate) run_queue_len: usize,
    pub(crate) next_resource: u64,
    pub(crate) next_capability: u64,
    pub(crate) next_task: u64,
    pub(crate) next_sequence: u64,
}
```

Initialize in `new()`:

```rust
run_queue: [RunQueueEntry::empty(); RUN_QUEUE],
run_queue_len: 0,
```

- [ ] **Step 4: Update all core impl generics**

For each impl in these files, change `KernelCore<RESOURCES, CAPS, EVENTS, TASKS>` to `KernelCore<RESOURCES, CAPS, EVENTS, TASKS, RUN_QUEUE>` and add `const RUN_QUEUE: usize` to the impl generic list:

- `authorization.rs`
- `capability_store.rs`
- `event_log.rs`
- `lookup.rs`
- `resource_store.rs`
- `task_store.rs`
- `core.rs`

Update core test aliases:

```rust
type TestCore = KernelCore<4, 4, 16, 4, 4>;
```

Update one-off core test instances, for example:

```rust
let mut core = KernelCore::<4, 4, 8, 1, 1>::new();
```

- [ ] **Step 5: Export run queue modules**

In `crates/agent-kernel-core/src/lib.rs`, add:

```rust
mod run_queue;
mod scheduler;
pub use run_queue::RunQueueEntry;
```

- [ ] **Step 6: Implement scheduler methods**

Create `crates/agent-kernel-core/src/scheduler.rs`:

```rust
//! Fixed-capacity FIFO task scheduler.
//!
//! This module belongs to `agent-kernel-core`. It owns enqueue, dispatch, and
//! yield behavior for accepted tasks. It performs deterministic queue mutation,
//! records scheduler events, and does not grant resource authority.

use crate::{
    AgentId, Event, EventKind, KernelCore, KernelError, RunQueueEntry, TaskId, TaskStatus,
};

impl<
        const RESOURCES: usize,
        const CAPS: usize,
        const EVENTS: usize,
        const TASKS: usize,
        const RUN_QUEUE: usize,
    > KernelCore<RESOURCES, CAPS, EVENTS, TASKS, RUN_QUEUE>
{
    pub fn enqueue_task(&mut self, agent: AgentId, task: TaskId) -> Result<Event, KernelError> {
        let task_record = self.find_runnable_task(agent, task)?;
        self.ensure_not_queued(task)?;
        self.ensure_run_queue_capacity()?;
        self.ensure_scheduler_event_capacity()?;

        self.run_queue[self.run_queue_len] = RunQueueEntry { task, agent };
        self.run_queue_len += 1;
        self.record_scheduler_event(EventKind::TaskQueued, agent, task, task_record.resource)
    }

    pub fn dispatch_next(&mut self, agent: AgentId) -> Result<TaskId, KernelError> {
        if self.run_queue_len == 0 {
            return Err(KernelError::RunQueueEmpty);
        }

        let entry = self.run_queue[0];
        if entry.agent != agent {
            return Err(KernelError::TaskNotRunnable);
        }
        let task_record = self.find_task(entry.task)?;
        self.ensure_scheduler_event_capacity()?;

        self.shift_run_queue_left();
        self.record_scheduler_event(EventKind::TaskDispatched, agent, entry.task, task_record.resource)?;
        Ok(entry.task)
    }

    pub fn yield_task(&mut self, agent: AgentId, task: TaskId) -> Result<Event, KernelError> {
        let task_record = self.find_runnable_task(agent, task)?;
        self.ensure_not_queued(task)?;
        self.ensure_run_queue_capacity()?;
        self.ensure_scheduler_event_capacity()?;

        self.run_queue[self.run_queue_len] = RunQueueEntry { task, agent };
        self.run_queue_len += 1;
        self.record_scheduler_event(EventKind::TaskYielded, agent, task, task_record.resource)
    }

    pub fn run_queue(&self) -> &[RunQueueEntry] {
        &self.run_queue[..self.run_queue_len]
    }

    fn find_runnable_task(
        &self,
        agent: AgentId,
        task: TaskId,
    ) -> Result<crate::Task, KernelError> {
        let task_record = self.find_task(task)?;
        if task_record.status != TaskStatus::Accepted || task_record.assignee != Some(agent) {
            return Err(KernelError::TaskNotRunnable);
        }
        Ok(task_record)
    }

    fn ensure_not_queued(&self, task: TaskId) -> Result<(), KernelError> {
        if self.run_queue().iter().any(|entry| entry.task == task) {
            Err(KernelError::TaskAlreadyQueued)
        } else {
            Ok(())
        }
    }

    fn ensure_run_queue_capacity(&self) -> Result<(), KernelError> {
        if self.run_queue_len >= RUN_QUEUE {
            Err(KernelError::RunQueueFull)
        } else {
            Ok(())
        }
    }

    fn ensure_scheduler_event_capacity(&self) -> Result<(), KernelError> {
        if self.event_len >= EVENTS {
            Err(KernelError::EventLogFull)
        } else {
            Ok(())
        }
    }

    fn record_scheduler_event(
        &mut self,
        kind: EventKind,
        agent: AgentId,
        task: TaskId,
        resource: crate::ResourceId,
    ) -> Result<Event, KernelError> {
        self.record(Event {
            sequence: self.next_sequence,
            agent,
            kind,
            resource: Some(resource),
            capability: None,
            action: None,
            operation: None,
            checkpoint: None,
            task: Some(task),
            target_agent: None,
        })
    }

    fn shift_run_queue_left(&mut self) {
        let last = self.run_queue_len - 1;
        let mut index = 0;
        while index < last {
            self.run_queue[index] = self.run_queue[index + 1];
            index += 1;
        }
        self.run_queue[last] = RunQueueEntry::empty();
        self.run_queue_len -= 1;
    }
}
```

- [ ] **Step 7: Run core scheduler tests**

Run:

```bash
PATH="$HOME/.cargo/bin:$PATH" RUSTC="$(rustup which rustc --toolchain nightly)" rustup run nightly cargo test -p agent-kernel-core scheduler
```

Expected: scheduler tests pass after compile fixes.

## Task 3: Facade, Boot, X86, And Existing Tests

**Files:**
- Modify: `crates/agent-kernel/src/lib.rs`
- Modify: `crates/agent-kernel/tests/kernel_facade.rs`
- Modify: `crates/agent-kernel-boot/src/lib.rs`
- Modify: `crates/agent-kernel-boot/tests/boot_flow.rs`
- Modify: `crates/agent-kernel-x86_64/src/main.rs`
- Modify existing tests using `KernelCore`, `AgentKernel`, or `BootedKernel` generic parameters.

- [ ] **Step 1: Mirror `RUN_QUEUE` in facade**

Change `AgentKernel` generics to:

```rust
pub struct AgentKernel<
    const RESOURCES: usize,
    const CAPS: usize,
    const EVENTS: usize,
    const TASKS: usize,
    const RUN_QUEUE: usize,
> {
    core: KernelCore<RESOURCES, CAPS, EVENTS, TASKS, RUN_QUEUE>,
}
```

Update impl and `Default` generics the same way.

- [ ] **Step 2: Add scheduler syscalls**

Add imports:

```rust
RunQueueEntry,
```

Add methods:

```rust
pub fn sys_enqueue_task(
    &mut self,
    agent: AgentId,
    task: TaskId,
) -> Result<Event, KernelError> {
    self.core.enqueue_task(agent, task)
}

pub fn sys_dispatch_next(&mut self, agent: AgentId) -> Result<TaskId, KernelError> {
    self.core.dispatch_next(agent)
}

pub fn sys_yield_task(&mut self, agent: AgentId, task: TaskId) -> Result<Event, KernelError> {
    self.core.yield_task(agent, task)
}

pub fn run_queue(&self) -> &[RunQueueEntry] {
    self.core.run_queue()
}
```

- [ ] **Step 3: Add facade scheduler tests**

In `crates/agent-kernel/tests/kernel_facade.rs`, update the alias:

```rust
type TestKernel = AgentKernel<4, 6, 32, 6, 4>;
```

Add imports:

```rust
RunQueueEntry,
```

Add test:

```rust
#[test]
fn scheduler_syscalls_enqueue_dispatch_and_yield_tasks() {
    let mut kernel = TestKernel::new();
    let owner = AgentId::new(200);
    let first_agent = AgentId::new(201);
    let second_agent = AgentId::new(202);
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
    let first = kernel
        .sys_create_task(owner, owner_capability, resource)
        .expect("first task should be created");
    let second = kernel
        .sys_create_task(owner, owner_capability, resource)
        .expect("second task should be created");
    kernel
        .sys_delegate_task(owner, owner_capability, first, first_agent)
        .expect("first task should delegate");
    kernel
        .sys_delegate_task(owner, owner_capability, second, second_agent)
        .expect("second task should delegate");
    kernel
        .sys_accept_task(first_agent, first)
        .expect("first task should accept");
    kernel
        .sys_accept_task(second_agent, second)
        .expect("second task should accept");

    kernel
        .sys_enqueue_task(first_agent, first)
        .expect("first task should enqueue");
    kernel
        .sys_yield_task(second_agent, second)
        .expect("second task should yield into queue");
    let dispatched = kernel
        .sys_dispatch_next(first_agent)
        .expect("first task should dispatch");

    assert_eq!(dispatched, first);
    assert_eq!(
        kernel.run_queue(),
        &[RunQueueEntry {
            task: second,
            agent: second_agent
        }]
    );
    assert_eq!(
        kernel.events().last().expect("dispatch event should exist").kind,
        EventKind::TaskDispatched
    );
}
```

- [ ] **Step 4: Update boot and x86 generics and event matches**

Update `BootedKernel` to carry `RUN_QUEUE`:

```rust
pub struct BootedKernel<
    const RESOURCES: usize,
    const CAPS: usize,
    const EVENTS: usize,
    const TASKS: usize,
    const RUN_QUEUE: usize,
> {
    kernel: AgentKernel<RESOURCES, CAPS, EVENTS, TASKS, RUN_QUEUE>,
    report: BootReport,
}
```

Update boot tests and x86 entry from:

```rust
BootedKernel::<8, 8, 16, 4>
```

to:

```rust
BootedKernel::<8, 8, 16, 4, 4>
```

Add x86 serial event labels:

```rust
EventKind::TaskQueued => serial_write_line("task_queued"),
EventKind::TaskDispatched => serial_write_line("task_dispatched"),
EventKind::TaskYielded => serial_write_line("task_yielded"),
```

- [ ] **Step 5: Update all generic arity in tests**

Use `rg "KernelCore::<|AgentKernel::<|BootedKernel::<|KernelCore<|AgentKernel<|BootedKernel<" -n crates` and update every four-parameter kernel type to five parameters by adding a run queue capacity.

Use these defaults:

- core unit/integration tests: `4`
- boot/x86: `4`
- supervisor: `8`

- [ ] **Step 6: Run workspace tests**

Run:

```bash
PATH="$HOME/.cargo/bin:$PATH" RUSTC="$(rustup which rustc --toolchain nightly)" rustup run nightly cargo test --workspace
```

Expected: all tests pass after generic arity, facade, boot, and x86 event updates.

## Task 4: Supervisor Flow And README

**Files:**
- Modify: `crates/agent-supervisor/src/main.rs`
- Modify: `crates/agent-supervisor/tests/supervisor_flow.rs`
- Modify: `README.md`

- [ ] **Step 1: Update supervisor kernel capacity**

Use:

```rust
let mut kernel = AgentKernel::<8, 8, 24, 8, 8>::new();
```

- [ ] **Step 2: Add enqueue and dispatch to supervisor task flow**

After task acceptance, call:

```rust
kernel
    .sys_enqueue_task(target_agent, task)
    .expect("target agent should enqueue accepted task");
let dispatched = kernel
    .sys_dispatch_next(target_agent)
    .expect("target agent should dispatch next task");
assert_eq!(dispatched, task);
```

Then complete and verify as before.

- [ ] **Step 3: Add supervisor event formatting**

Add match arms:

```rust
EventKind::TaskQueued => format_task_event(event, "task_queued"),
EventKind::TaskDispatched => format_task_event(event, "task_dispatched"),
EventKind::TaskYielded => format_task_event(event, "task_yielded"),
```

- [ ] **Step 4: Update supervisor output test**

Update `crates/agent-supervisor/tests/supervisor_flow.rs` to expect:

```text
event[6] task_created agent=1 resource=1 task=1
event[7] delegation agent=1 resource=1 task=1 target_agent=2
event[8] task_accepted agent=2 resource=1 task=1
event[9] task_queued agent=2 resource=1 task=1
event[10] task_dispatched agent=2 resource=1 task=1
event[11] task_completed agent=2 resource=1 task=1
event[12] task_verified agent=1 resource=1 task=1
```

- [ ] **Step 5: Update README**

Update current behavior to say the kernel now owns a FIFO run queue and
dispatches accepted tasks before completion.

Update expected supervisor output with the same event sequence from Step 4.

- [ ] **Step 6: Run supervisor**

Run:

```bash
PATH="$HOME/.cargo/bin:$PATH" RUSTC="$(rustup which rustc --toolchain nightly)" rustup run nightly cargo run -p agent-supervisor
```

Expected: supervisor output includes `task_queued` and `task_dispatched` before `task_completed`.

## Task 5: Verification And Publish

**Files:**
- Review all changed files.

- [ ] **Step 1: Format check**

Run:

```bash
PATH="$HOME/.cargo/bin:$PATH" rustup run nightly cargo fmt --check
```

Expected: exit 0.

- [ ] **Step 2: Workspace tests**

Run:

```bash
PATH="$HOME/.cargo/bin:$PATH" RUSTC="$(rustup which rustc --toolchain nightly)" rustup run nightly cargo test --workspace
```

Expected: exit 0.

- [ ] **Step 3: Supervisor run**

Run:

```bash
PATH="$HOME/.cargo/bin:$PATH" RUSTC="$(rustup which rustc --toolchain nightly)" rustup run nightly cargo run -p agent-supervisor
```

Expected: output includes task queue and dispatch events in order.

- [ ] **Step 4: QEMU boot**

Run:

```bash
scripts/run-qemu.sh
```

Expected: serial output reaches `SUPERVISOR_HANDOFF_READY`.

- [ ] **Step 5: Scope and line-size check**

Run:

```bash
wc -l crates/agent-kernel-core/src/*.rs crates/agent-kernel-core/tests/*.rs crates/agent-kernel/src/lib.rs crates/agent-supervisor/src/main.rs
git diff --check
git status -sb
```

Expected: no hard-limit file sizes, no whitespace errors, only intended scheduler files changed.

- [ ] **Step 6: Commit and push**

Run:

```bash
git add README.md crates/agent-kernel-core crates/agent-kernel crates/agent-kernel-boot crates/agent-kernel-x86_64 crates/agent-supervisor docs/superpowers/plans/2026-07-02-scheduler-run-queue-v0.md
git commit -m "feat: add scheduler run queue"
git push origin main
```

Expected: commit pushed to `origin/main`.
