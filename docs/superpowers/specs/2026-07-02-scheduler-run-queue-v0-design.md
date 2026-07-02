# Scheduler Run Queue V0 Design

## Purpose

Scheduler Run Queue V0 gives Agent Kernel a deterministic way to decide which
accepted task is ready to run next. The current kernel can create, delegate,
accept, complete, verify, and cancel tasks, but the supervisor still chooses
which task to work on. This design adds a kernel-owned ready queue and dispatch
events so task execution order becomes an inspectable kernel primitive.

The design is intentionally small: a fixed-capacity FIFO run queue with explicit
enqueue, dispatch, and yield operations. Priority, policy, deadlines, and
authority-aware scheduling are deferred until the kernel has stronger authority
and resource models.

## Selected Approach

Use a deterministic FIFO run queue inside `agent-kernel-core`.

Alternatives considered:

- Priority scheduler: useful later, but priority would introduce policy before
  tasks have intents, costs, or risk classes.
- Authority-aware scheduler: more AgentOS-specific, but it needs child
  capability derivation and risk modeling that do not exist yet.
- Supervisor-owned scheduling: easy, but it keeps execution ordering outside the
  kernel and makes scheduling impossible to replay from kernel state.

FIFO is the right V0 because it is deterministic, no_std-friendly, and creates a
clear scheduling boundary without pretending to solve policy.

## Architecture Placement

`agent-kernel-core` owns:

- fixed-capacity run queue storage,
- queue entry model,
- enqueue validation,
- FIFO dispatch,
- task yield behavior,
- scheduler event emission,
- capacity and state errors.

`agent-kernel` owns:

- syscall-style wrappers over scheduler APIs,
- no direct queue mutation outside `agent-kernel-core`.

`agent-supervisor` owns:

- demonstration of accepted tasks being enqueued and dispatched,
- printing the resulting event sequence,
- no scheduler policy beyond calling kernel syscalls.

Boot crates stay unchanged in behavior for this phase. They must compile with
the new queue capacity parameter, but early boot will still emit only
observe/action/verify handoff events.

## Core Data Model

Add `RunQueueEntry`:

```rust
pub struct RunQueueEntry {
    pub task: TaskId,
    pub agent: AgentId,
}
```

The entry stores the task and the agent expected to run it. In V0 this is the
task assignee. Future versions may include resource class, priority, deadline,
or capability snapshot IDs.

`KernelCore` gains a fifth capacity parameter:

```rust
KernelCore<
    const RESOURCES: usize,
    const CAPS: usize,
    const EVENTS: usize,
    const TASKS: usize,
    const RUN_QUEUE: usize,
>
```

`AgentKernel` and `BootedKernel` mirror the same capacity. This is intentional:
the run queue is first-class kernel state, not a host-side helper.

The queue itself is a fixed array:

```rust
run_queue: [RunQueueEntry; RUN_QUEUE],
run_queue_len: usize,
```

`RunQueueEntry` will provide a crate-private `empty()` value using zero IDs so
the fixed array can be initialized without heap allocation. Public queue
inspection returns only the initialized prefix.

V0 can remove the first queue item by shifting remaining entries left. This is
simple, deterministic, and acceptable for a small fixed-capacity prototype.

## Operations

Add core methods:

- `enqueue_task(agent, task) -> Result<Event, KernelError>`
- `dispatch_next(agent) -> Result<TaskId, KernelError>`
- `yield_task(agent, task) -> Result<Event, KernelError>`
- `run_queue() -> &[RunQueueEntry]`

`enqueue_task`:

- requires the task to exist,
- requires `task.status == TaskStatus::Accepted`,
- requires `task.assignee == Some(agent)`,
- rejects duplicate queue entries for the same task,
- records `TaskQueued`.

`dispatch_next`:

- fails with `RunQueueEmpty` if no tasks are queued,
- pops the oldest queued entry,
- requires the queued entry agent to match the caller,
- records `TaskDispatched`,
- returns the dispatched `TaskId`.

`yield_task`:

- requires the task to exist,
- requires `task.status == TaskStatus::Accepted`,
- requires `task.assignee == Some(agent)`,
- appends the task to the back of the queue,
- records `TaskYielded`.

Yield is distinct from enqueue because it means the task was already selected or
running from the scheduler's perspective. V0 will not model "running" as a task
status because the current task lifecycle uses `Accepted -> Completed`; adding a
separate running state is deferred until execution slices are modeled.

## Event Model

Extend `EventKind` with:

- `TaskQueued`
- `TaskDispatched`
- `TaskYielded`

Every scheduler mutation emits exactly one event:

- enqueue emits `TaskQueued`
- dispatch emits `TaskDispatched`
- yield emits `TaskYielded`

Dispatch is a kernel-visible lifecycle step even though it does not alter
`TaskStatus` in V0. The event log is the source of truth for which task was
selected to run.

## Error Handling

Add explicit errors:

- `RunQueueFull`
- `RunQueueEmpty`
- `TaskNotRunnable`
- `TaskAlreadyQueued`

Invalid enqueue/yield/dispatch attempts must leave both queue state and event
log unchanged.

`TaskNotRunnable` covers tasks that are not accepted, have no assignee, or are
being scheduled by the wrong agent.

## Determinism And Authority

The scheduler must not call models, inspect prompts, use wall-clock time, or
perform host I/O. All scheduling decisions are deterministic functions of kernel
state and syscall arguments.

Scheduling does not grant authority. Completing or verifying a dispatched task
still uses existing capability checks. This preserves the split between "kernel
selected this task" and "agent is authorized to mutate the resource."

## Tests

Implementation must start with failing tests.

Core tests:

- enqueue accepted task records `TaskQueued` and stores FIFO entry
- enqueue rejects unaccepted task with `TaskNotRunnable`
- enqueue rejects wrong agent with `TaskNotRunnable`
- enqueue rejects duplicate task with `TaskAlreadyQueued`
- enqueue returns `RunQueueFull` when full
- dispatch pops the oldest queued task and records `TaskDispatched`
- dispatch from empty queue returns `RunQueueEmpty`
- dispatch rejects wrong agent without changing queue or event log
- yield requeues an accepted task at the back and records `TaskYielded`

Facade tests:

- syscall wrappers preserve enqueue, dispatch, and yield behavior
- facade exposes queue inspection without mutable access

Supervisor test:

- host flow prints create, delegate, accept, enqueue, dispatch, complete, and
  verify events in order.

Full verification:

```bash
PATH="$HOME/.cargo/bin:$PATH" rustup run nightly cargo fmt --check
PATH="$HOME/.cargo/bin:$PATH" RUSTC="$(rustup which rustc --toolchain nightly)" rustup run nightly cargo test --workspace
PATH="$HOME/.cargo/bin:$PATH" RUSTC="$(rustup which rustc --toolchain nightly)" rustup run nightly cargo run -p agent-supervisor
scripts/run-qemu.sh
```

## Compatibility Impact

This design intentionally changes generic parameters for `KernelCore`,
`AgentKernel`, and `BootedKernel` by adding `RUN_QUEUE`. All internal crates and
tests should be updated in one implementation commit.

There is no POSIX, Linux syscall, or shell compatibility impact. This remains an
AgentOS-native scheduling primitive.

## Deferred Work

V0 does not include:

- priority,
- preemption,
- time slices,
- parallel execution,
- load balancing,
- running task status,
- per-agent queues,
- task deadlines,
- policy plugins,
- authority-aware dispatch,
- persistence/replay from event log into queue state.

The next scheduler step after V0 should be either a `Running` execution state or
capability derivation for delegated work, depending on whether execution slices
or authority splitting becomes the sharper bottleneck.
