# Running Task State V0 Design

## Purpose

Running Task State V0 makes dispatch a real kernel execution boundary instead
of only an event-log marker. The current scheduler can enqueue and dispatch
accepted tasks, but a task can still move directly from `Accepted` to
`Completed` through `complete_task`. That leaves execution order inspectable but
not enforced by task state.

This design adds `TaskStatus::Running` and changes the lifecycle so an accepted
task must be dispatched before it can be completed. The scheduler remains small:
no time slices, preemption, policy plugins, or authority-aware ordering. The
kernel simply records that it selected a task for execution and makes that fact
part of the task state machine.

## Selected Approach

Add a `Running` task status and make `dispatch_next` perform the transition
`Accepted -> Running`.

Alternatives considered:

- Keep dispatch event-only: simplest, but it lets agents bypass the scheduler
  and complete accepted tasks directly.
- Add a separate running table: useful later for slices, leases, and recovery,
  but it duplicates task state before V0 needs a separate execution record.
- Add preemptive time slices now: too much policy and timing for the current
  deterministic no_std kernel core.

`Running` is the right V0 because it is a small native kernel primitive that
turns execution selection into state without introducing clocks, host I/O, or
policy.

## Architecture Placement

`agent-kernel-core` owns:

- the new `TaskStatus::Running` value,
- dispatch state transition validation,
- yield transition validation,
- completion precondition changes,
- cancellation rules for running tasks,
- event-log ordering for dispatch and yield.

`agent-kernel` owns:

- no new syscall names in this phase,
- existing scheduler syscall wrappers that now expose stronger core semantics.

`agent-supervisor` owns:

- the same user-space flow as scheduler V0: accept, enqueue, dispatch,
  complete, verify,
- output formatting only.

Boot crates stay unchanged in behavior. They must continue compiling with the
existing `RUN_QUEUE` capacity parameter, but early boot still emits only
observation, action, and verification handoff events.

## Task State Model

Extend `TaskStatus`:

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

Lifecycle after this change:

```text
Created -> Delegated -> Accepted -> Running -> Completed -> Verified
                         \          \
                          \          -> Cancelled
                           -> Cancelled
Running -> Accepted through yield_task
```

`Running` means the kernel has dispatched the task to its assignee. It does not
mean an LLM or host process is running inside kernel space. The supervisor still
performs reasoning and work outside the kernel.

V0 allows multiple tasks to be `Running` at once because the current kernel has
no execution leases, per-agent active-task slots, or preemption. A future
execution-slice model can restrict or lease running work if needed.

## Operation Semantics

`enqueue_task(agent, task)`:

- unchanged from Scheduler Run Queue V0,
- requires `task.status == TaskStatus::Accepted`,
- rejects `Running`, `Completed`, `Verified`, and `Cancelled` tasks with
  `TaskNotRunnable`.

`dispatch_next(agent)`:

- fails with `RunQueueEmpty` if the queue is empty,
- inspects the oldest queue entry,
- requires the queued entry agent to match the caller,
- requires the task to still be `Accepted` and assigned to the caller,
- checks event capacity before mutation,
- removes the queue head,
- changes task status from `Accepted` to `Running`,
- records `TaskDispatched`,
- returns the dispatched `TaskId`.

`yield_task(agent, task)`:

- requires the task to exist,
- requires `task.status == TaskStatus::Running`,
- requires `task.assignee == Some(agent)`,
- rejects duplicate queue entries for the same task,
- checks queue capacity and event capacity before mutation,
- changes task status from `Running` to `Accepted`,
- appends the task to the back of the run queue,
- records `TaskYielded`.

`complete_task(agent, capability, task)`:

- requires the task to exist,
- requires action capability as before,
- requires `task.status == TaskStatus::Running`,
- requires `task.assignee == Some(agent)`,
- changes task status from `Running` to `Completed`,
- records `TaskCompleted`.

`cancel_task(agent, capability, task)`:

- keeps existing rollback capability requirements,
- may cancel `Created`, `Delegated`, `Accepted`, `Running`, and `Completed`
  tasks,
- changes the task status to `Cancelled`,
- records `TaskCancelled`.

`accept_task`, `verify_task`, and task allocation semantics stay unchanged.

## Event Model

No new event kinds are required.

Existing scheduler events gain stronger state meaning:

- `TaskQueued`: accepted task entered the ready queue.
- `TaskDispatched`: accepted task became running.
- `TaskYielded`: running task returned to accepted state and re-entered the
  ready queue.

Every mutating operation continues to emit exactly one event after successful
state mutation. Invalid attempts must leave task state, run queue state, and
event log unchanged.

## Error Handling

No new error variants are required.

Existing errors are used as follows:

- `TaskNotRunnable`: task is not in the state required for enqueue, dispatch,
  or yield; or the caller is not the assigned agent.
- `TaskStatusMismatch`: lifecycle operation is valid for tasks generally but
  the current task is in the wrong lifecycle state, such as completing an
  accepted task before dispatch.
- `TaskAgentMismatch`: task lifecycle caller is not the task assignee where a
  lifecycle operation requires the assignee.
- `RunQueueEmpty`, `RunQueueFull`, and `TaskAlreadyQueued`: unchanged.

The important behavioral change is that completing an accepted-but-not-running
task now fails with `TaskStatusMismatch`.

## Determinism And Authority

The running transition is deterministic and derived only from kernel state and
syscall arguments. The scheduler still does not call models, inspect prompts,
read wall-clock time, perform host I/O, or grant authority.

Dispatch does not create capabilities. Completing a running task still requires
an explicit action capability for the task resource. This preserves the boundary
between execution selection and resource authority.

## Tests

Implementation must start with failing tests.

Core tests:

- dispatching a queued accepted task changes status to `Running` and records
  `TaskDispatched`.
- completing an accepted task before dispatch fails with `TaskStatusMismatch`
  and leaves task state and event log unchanged.
- completing a running task changes status to `Completed`.
- yielding a running task changes status back to `Accepted`, appends it to the
  queue, and records `TaskYielded`.
- yielding an accepted-but-not-running task fails with `TaskNotRunnable` and
  leaves state unchanged.
- cancelling a running task changes status to `Cancelled` and prevents later
  completion.

Facade tests:

- syscall flow requires dispatch before completion.
- syscall yield returns a running task to the visible run queue.

Supervisor test:

- existing host flow still prints accept, enqueue, dispatch, complete, and
  verify events in order.

Full verification:

```bash
PATH="$HOME/.cargo/bin:$PATH" rustup run nightly cargo fmt --check
PATH="$HOME/.cargo/bin:$PATH" RUSTC="$(rustup which rustc --toolchain nightly)" rustup run nightly cargo test --workspace
PATH="$HOME/.cargo/bin:$PATH" RUSTC="$(rustup which rustc --toolchain nightly)" rustup run nightly cargo run -p agent-supervisor
scripts/run-qemu.sh
```

## Compatibility Impact

This design changes task lifecycle semantics. Existing code that completes an
accepted task without dispatch must insert `enqueue_task` and `dispatch_next`
first.

There is no POSIX, Linux syscall, shell, or legacy compatibility impact. This is
an AgentOS-native execution-state primitive.

## Deferred Work

V0 does not include:

- per-agent active task slots,
- preemption,
- time slices,
- execution leases,
- watchdogs,
- persistence/replay from event log into running state,
- capability derivation for delegated work,
- authority-aware scheduling.

The next step after Running State V0 should be delegated capability derivation,
because the kernel will then have both an execution boundary and a clearer need
for task-scoped authority.
