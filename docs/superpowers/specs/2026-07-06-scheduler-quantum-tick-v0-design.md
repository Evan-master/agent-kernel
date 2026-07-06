# Scheduler Quantum Tick V0 Design

## Purpose

Scheduler Quantum Tick V0 adds deterministic runtime accounting to kernel
tasks. It is not a host timer, thread scheduler, interrupt controller, wall
clock, sleep API, or POSIX process quantum. It gives the Agent Kernel a native
way to dispatch a task with a finite quantum, advance it by explicit ticks, and
preempt it back into the run queue when that quantum is exhausted.

## Scope

V0 provides:

- per-task `run_ticks` and `quantum_remaining` counters,
- `dispatch_next_with_quantum(agent, quantum)` for dispatching the next accepted
  task with an explicit nonzero quantum,
- existing `dispatch_next(agent)` mapped to a default quantum of `1`,
- `tick_task(agent, task)` for deterministic single-tick advancement,
- replayable `TaskTicked` and `TaskQuantumExpired` events,
- scheduler event fields for the cumulative task tick count and remaining
  quantum,
- facade syscalls, supervisor output, QEMU event labels, and documentation.

V0 intentionally does not provide priorities, deadlines, wall-clock time,
preemptive interrupts, sleeping, timers, CPU cores, blocking syscalls, or host
thread integration.

## Core Model

```rust
pub struct Task {
    pub id: TaskId,
    pub intent: IntentId,
    pub owner: AgentId,
    pub resource: ResourceId,
    pub assignee: Option<AgentId>,
    pub delegated_capability: Option<CapabilityId>,
    pub status: TaskStatus,
    pub run_ticks: u64,
    pub quantum_remaining: u64,
}
```

`TaskDispatched` records the assigned quantum. `TaskTicked` records one
deterministic tick and leaves the task running when more quantum remains.
`TaskQuantumExpired` records the final tick, moves the task back to
`TaskStatus::Accepted`, and appends it to the back of the fixed-capacity run
queue.

## Authority And Ordering

Only the assigned active agent can tick a running task. Ticking a non-running
task, a task assigned to another agent, or an unregistered/suspended/retired
agent fails without mutation. Dispatching with `quantum == 0` returns
`TaskQuantumInvalid`.

Every successful tick appends exactly one event. If a tick would expire the
quantum, the kernel checks run queue capacity and event capacity before
mutating task counters or status. Event-log-full, run-queue-full, invalid
quantum, invalid task state, and inactive-agent failures leave task state,
queue state, and event logs unchanged.

## Test Evidence

Tests must prove:

- `dispatch_next` assigns the default quantum of `1`,
- `dispatch_next_with_quantum` assigns the requested nonzero quantum,
- `tick_task` increments `run_ticks`, decrements `quantum_remaining`, and records
  `TaskTicked` while quantum remains,
- the final tick records `TaskQuantumExpired`, sets status back to `Accepted`,
  and requeues the task at the back,
- zero quantum dispatch fails without mutation,
- ticking non-running tasks and wrong-agent tasks fails without mutation,
- event-log-full and run-queue-full expiry paths are atomic,
- facade syscalls expose explicit quantum dispatch and tick behavior.
