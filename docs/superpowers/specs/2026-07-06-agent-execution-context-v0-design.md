# Agent Execution Context V0 Design

## Purpose

Agent Execution Context V0 makes agent runtime state a first-class kernel
record. The kernel already has agents, tasks, a run queue, and deterministic
ticks, but the currently running task is only represented on the task itself.
V0 adds one execution context per registered agent so the kernel can answer:
"what is this agent executing or blocked on right now?"

## Scope

V0 provides:

- `AgentExecutionState::{Idle, Running, Waiting, Faulted}`,
- `AgentExecutionContext { agent, state, task, run_ticks, quantum_remaining }`,
- an execution context created atomically with every registered agent,
- read-only context inspection through `execution_contexts()` and
  `execution_context(agent)`,
- scheduler/fault/signal/task lifecycle updates that keep context state aligned
  with task state,
- facade inspection from `agent-kernel`,
- tests and README coverage.

V0 does not execute code, store model prompts, add host threads, define a
process abstraction, add memory address spaces, or replace the existing task
store and run queue. It also does not add a separate context capacity: context
capacity is exactly `AGENTS`, with one context per registered agent.

## Core Model

```rust
pub enum AgentExecutionState {
    Idle,
    Running,
    Waiting,
    Faulted,
}

pub struct AgentExecutionContext {
    pub agent: AgentId,
    pub state: AgentExecutionState,
    pub task: Option<TaskId>,
    pub run_ticks: u64,
    pub quantum_remaining: u64,
}
```

`Idle` contexts have `task: None`, `run_ticks: 0`, and
`quantum_remaining: 0`. `Running`, `Waiting`, and `Faulted` contexts carry the
task they are tied to. `Running` also carries the current task tick snapshot and
remaining quantum.

## State Transitions

Agent registration creates an idle context at the same index as the agent
record. This mutation is covered by the existing `AgentRegistered` event.

Scheduler and lifecycle methods update contexts as follows:

- `dispatch_next_with_quantum(agent, quantum)` requires the agent context to be
  idle, sets it to `Running`, and records `TaskDispatched`.
- `tick_task` keeps the context `Running` with updated tick and quantum values
  while quantum remains, then clears it to `Idle` when `TaskQuantumExpired`
  requeues the task.
- `yield_task` clears the context to `Idle` when the task returns to
  `Accepted`.
- `wait_task` sets the context to `Waiting` when the task enters `Waiting`.
- `emit_signal` clears the waiting agent context to `Idle` when the task is
  woken back to `Accepted`.
- `fault_task` sets the context to `Faulted` when a running task traps.
- `recover_faulted_task` and `FaultPolicyAction::RecoverTask` clear the faulted
  agent context to `Idle` when the task returns to `Accepted`.
- `complete_task` clears the context to `Idle` when a running task completes.
- `cancel_task` clears the context to `Idle` when the cancelled task was the
  context's current task.

An agent may not dispatch a second task while its context is `Running`,
`Waiting`, or `Faulted`. V0 returns `KernelError::ExecutionContextBusy` before
mutating task state or the run queue.

## Event Model

V0 does not add a new event kind. Context changes are deterministic companion
state for existing task and agent events:

- `AgentRegistered`,
- `TaskDispatched`,
- `TaskTicked`,
- `TaskQuantumExpired`,
- `TaskYielded`,
- `TaskWaiting`,
- `TaskWoken`,
- `TaskFaulted`,
- `TaskFaultRecovered`,
- `TaskCompleted`,
- `TaskCancelled`.

This keeps the event log compact while preserving replayability. Every context
mutation happens in the same operation that records one of those events.

## Atomicity And Authority

All context updates happen after existing authority and capacity checks. Failure
paths leave task state, run queue state, event log state, and execution context
state unchanged. Agent registration validates duplicate agents, agent capacity,
and event capacity before writing either the agent record or the context record.

## Test Evidence

Tests must prove:

- registering an agent creates an idle execution context,
- dispatch, tick, quantum expiry, yield, wait/wake, fault/recover, complete, and
  cancel keep context state aligned with task state,
- dispatch rejects a second task for an agent with a busy context without
  popping the run queue or changing context state,
- registration failures do not create orphan contexts,
- the facade exposes execution contexts without letting callers mutate kernel
  internals.
