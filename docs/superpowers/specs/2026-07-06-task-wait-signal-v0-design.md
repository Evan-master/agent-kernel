# Task Wait Signal V0 Design

## Purpose

Task Wait Signal V0 adds an agent-native blocking primitive to the kernel. A
running task can wait on a typed signal scoped to a resource, and an authorized
agent can emit that signal to make the oldest matching waiter runnable again.
This is not a POSIX wait, fd readiness, or async runtime wrapper; it is a fixed
capacity kernel object with deterministic events.

## Scope

V0 provides:

- first-class `WaiterId`, `SignalKey`, `WaiterRecord`, and `SignalOutcome`
  types,
- fixed-capacity waiter storage owned by `KernelCore`,
- `wait_task(agent, capability, task, resource, signal)`,
- `emit_signal(agent, capability, resource, signal)`,
- `TaskWaiting`, `SignalEmitted`, and `TaskWoken` events,
- event fields for `waiter` and `signal`,
- facade syscalls, supervisor output, QEMU labels, tests, and documentation.

V0 intentionally does not provide timers, priorities, broadcast wakeups,
multi-condition waits, cancellation-specific wait events, cross-resource wait
sets, async runtimes, host callbacks, or model calls.

## Core Model

```rust
pub struct WaiterId(u64);
pub struct SignalKey(u64);

pub struct WaiterRecord {
    pub id: WaiterId,
    pub task: TaskId,
    pub agent: AgentId,
    pub resource: ResourceId,
    pub signal: SignalKey,
    pub active: bool,
}

pub struct SignalOutcome {
    pub signal_event: Event,
    pub woken_task: Option<TaskId>,
    pub wake_event: Option<Event>,
}
```

## Authority And Ordering

Waiting requires:

- active waiting agent,
- a running task assigned to that agent,
- task-scoped or root `Operation::Act` authority on the task resource,
- waiter store capacity,
- one event slot.

Waiting moves the task from `Running` to `Waiting`, creates an active waiter
record, and records `TaskWaiting`.

Signal emission requires:

- active emitter agent,
- root `Operation::Act` authority on the signal resource.

If no active waiter matches `(resource, signal)`, emission records one
`SignalEmitted` event and returns no woken task.

If a waiter matches, emission requires run queue capacity and two event slots.
It records `SignalEmitted`, marks the waiter inactive, moves the waited task
back to `Accepted`, appends it to the run queue, then records `TaskWoken`.

All capacity, authority, and status failures leave waiters, task state, run
queue state, and event logs unchanged.

## Test Evidence

Tests must prove:

- waiting records a waiter, blocks the running task, and records `TaskWaiting`,
- emitting a signal without waiters records only `SignalEmitted`,
- emitting a matching signal wakes the oldest waiter and enqueues the task,
- waiting requires a running assigned task and does not mutate otherwise,
- waiting store-full failures are atomic,
- signal emission requires act authority,
- signal run-queue-full and event-log-full failures are atomic,
- facade syscalls expose wait, emit, and waiter inspection.
