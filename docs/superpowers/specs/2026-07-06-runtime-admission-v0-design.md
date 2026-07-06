# Runtime Admission V0 Design

## Purpose

Runtime Admission V0 turns agent launch entries from passive audit records into
the kernel boundary that admits agents into execution. The kernel already
records `AgentLaunched`, but the scheduler still allows an active assignee to
enter the run queue without consulting that launch entry. V0 closes that gap:
capabilities answer "what may this agent touch," while launch admission answers
"where may this agent run."

## Scope

V0 provides:

- task-scoped launch entries for delegated workers,
- live admission checks against launch entry capability state,
- scheduler admission checks for enqueue, dispatch, ticks, yields, waits,
  faults, completion, and signal wakeups,
- facade syscall `sys_launch_task_agent`,
- supervisor flow coverage for launching the delegated worker before it runs,
- tests and README coverage.

V0 does not add address spaces, process identifiers, binary loading, preemptive
threads, host execution, or launch retirement. It does not require launch for
control-plane operations such as registration, capability grants, intent
declaration, task creation, delegation, verification, messaging, or resource
creation. Those remain capability-gated operations.

## Core Model

`AgentEntryRecord` gains a task scope:

```rust
pub struct AgentEntryRecord {
    pub agent: AgentId,
    pub resource: ResourceId,
    pub capability: CapabilityId,
    pub kind: AgentEntryKind,
    pub intent: Option<IntentId>,
    pub task: Option<TaskId>,
}
```

Existing `launch_agent` records `task: None`, producing a resource-scoped
entry. New `launch_task_agent` records `task: Some(task)`, producing a
task-scoped entry.

## Task-Scoped Launch

`launch_task_agent` takes:

```rust
launch_task_agent(
    agent: AgentId,
    capability: CapabilityId,
    task: TaskId,
    kind: AgentEntryKind,
) -> Result<Event, KernelError>
```

The operation validates:

- the agent is registered and active,
- the agent does not already have a launch entry,
- the task exists,
- the task has `assignee: Some(agent)`,
- the task is `Delegated` or `Accepted`,
- the capability authorizes `Operation::Act` for that exact task,
- one launch-entry slot is available,
- one event-log slot is available.

The launch event stores `resource`, `capability`, `intent`, `task`, and
`target_agent: Some(agent)`.

## Admission Rule

Before an agent can mutate task runtime state, the kernel must prove the agent
has a launch entry that covers the task:

- resource-scoped entry: entry resource must equal task resource, and the entry
  capability must still authorize root `Operation::Act` for that resource,
- task-scoped entry: entry task must equal the task, and the entry capability
  must still authorize task-scoped `Operation::Act` for that task.

Admission is checked by:

- `enqueue_task`,
- `dispatch_next_with_quantum`,
- `tick_task`,
- `yield_task`,
- `wait_task`,
- `fault_task`,
- `complete_task`,
- signal wakeup before requeueing a waiting task.

If the entry is missing, V0 returns `KernelError::AgentNotLaunched`. If a
task-scoped entry points at a different task, V0 returns
`KernelError::AgentEntryScopeMismatch`. If a launch capability is revoked, the
existing capability-chain errors surface and the runtime mutation is rejected.

## Event Model

V0 does not add a second admission event. Admission checks either permit an
existing task event to be recorded or reject the operation before mutation. The
new task-scoped launch path records the existing `AgentLaunched` event with the
task field set.

## Atomicity And Authority

Admission failures occur before run queue, task, waiter, fault, execution
context, or event-log mutation. This keeps failed admission invisible because
no kernel state changed. Successful runtime mutations are still covered by the
existing task, scheduler, signal, and fault events.

## Test Evidence

Tests must prove:

- resource-scoped launch admits task runtime operations on the same resource,
- task-scoped launch admits a delegated worker without broad root authority,
- unlaunched assignees cannot enqueue or dispatch tasks,
- a task-scoped entry cannot run a different task,
- revoking the launch capability blocks future runtime mutation,
- signal wakeup does not requeue a waiting task if the waiting agent's launch
  authority is no longer valid,
- facade and supervisor flows expose the new task-scoped launch path.
