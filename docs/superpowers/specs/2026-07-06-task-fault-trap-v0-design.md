# Task Fault Trap V0 Design

## Purpose

Task Fault Trap V0 makes task failure a first-class kernel event and record. It
is not a Rust panic, host exception, log line, POSIX signal, process exit code,
or supervisor-only convention. A running agent task can trap into the kernel,
record a deterministic fault, enter `TaskStatus::Faulted`, and later be
recovered by explicit rollback authority.

## Scope

V0 provides:

- first-class `FaultId`, `FaultKind`, and `FaultRecord` types,
- fixed-capacity fault storage owned by `KernelCore`,
- `fault_task(agent, task, kind, detail)` for an assigned running task to trap
  into the kernel,
- `recover_faulted_task(agent, capability, task)` for rollback-authorized
  recovery from faulted back to accepted,
- `TaskFaulted` and `TaskFaultRecovered` events,
- event fields for `fault`, `fault_kind`, and `fault_detail`,
- task field `last_fault`,
- facade syscalls, supervisor output, QEMU labels, and documentation.

V0 intentionally does not provide host stack traces, dynamic error messages,
panic unwinding, retry policies, automatic rollback execution, signal masks,
priority inheritance, or exception vectors.

## Core Model

```rust
pub struct FaultId(u64);

pub enum FaultKind {
    ExecutionTrap,
    AuthorityViolation,
    ResourceFault,
    VerificationFault,
}

pub struct FaultRecord {
    pub id: FaultId,
    pub task: TaskId,
    pub agent: AgentId,
    pub resource: ResourceId,
    pub kind: FaultKind,
    pub detail: u64,
}
```

`Task` gains `last_fault: Option<FaultId>`, and `TaskStatus` gains
`Faulted`.

## Authority And Ordering

Only the assigned active agent can fault its currently running task. Faulting
checks fault-store capacity and event-log capacity before mutation. A successful
fault appends one fault record, sets the task status to `Faulted`, clears
`quantum_remaining`, stores `last_fault`, and records `TaskFaulted`.

Recovery requires `Operation::Rollback` authority on the task resource. A
successful recovery moves the task back to `Accepted` while preserving
`last_fault` for inspection, and records `TaskFaultRecovered`.

Fault-store-full, event-log-full, inactive-agent, wrong-agent, invalid-status,
and missing-authority failures leave task state, fault records, run queue, and
event logs unchanged.

## Test Evidence

Tests must prove:

- faulting a running task records a fault, marks the task faulted, clears
  quantum, stores `last_fault`, and records `TaskFaulted`,
- recovering a faulted task requires rollback authority, moves it back to
  `Accepted`, and records `TaskFaultRecovered`,
- faulting an accepted task fails without mutation,
- faulting another agent's running task fails without mutation,
- fault-store-full and event-log-full fault paths are atomic,
- recovery without rollback authority fails without mutation,
- recovery event-log-full leaves the task faulted,
- facade syscalls expose fault and recovery behavior.
