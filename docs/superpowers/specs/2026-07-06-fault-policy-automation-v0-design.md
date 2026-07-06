# Fault Policy Automation V0 Design

## Purpose

Fault Policy Automation V0 makes fault handling policy a kernel object. Task
Fault Trap V0 records a fault, and Fault Handler Routing V0 can route it to an
agent. This V0 adds deterministic policy records so a rollback-authorized agent
can ask the kernel to apply the configured action for a fault without encoding
the decision in supervisor control flow.

## Scope

V0 provides:

- first-class `FaultPolicyId`, `FaultPolicyAction`, `FaultPolicyRecord`, and
  `FaultPolicyOutcome` types,
- fixed-capacity fault policy storage owned by `KernelCore`,
- `install_fault_policy(agent, capability, resource, kind, action)`,
- `apply_fault_policy(agent, capability, fault)`,
- `FaultPolicyInstalled` and `FaultPolicyApplied` events,
- event fields for `fault_policy` and `fault_policy_action`,
- facade syscalls, supervisor output, QEMU labels, and documentation.

V0 intentionally does not provide dynamic scripts, priorities, wildcard
policies, multiple chained policies, automatic invocation inside `fault_task`,
retry loops, model calls, or host callbacks.

## Core Model

```rust
pub struct FaultPolicyId(u64);

pub enum FaultPolicyAction {
    RouteToHandler,
    RecoverTask,
}

pub struct FaultPolicyRecord {
    pub id: FaultPolicyId,
    pub resource: ResourceId,
    pub kind: FaultKind,
    pub installer: AgentId,
    pub action: FaultPolicyAction,
}

pub struct FaultPolicyOutcome {
    pub action: FaultPolicyAction,
    pub message: Option<MessageId>,
    pub event: Event,
}
```

## Authority And Ordering

Installing a policy requires:

- active installer agent,
- `Operation::Rollback` authority on the resource,
- unused `(resource, fault_kind)` policy binding,
- policy store capacity,
- one event slot.

Applying a policy requires:

- active applying agent,
- an existing fault,
- the task still in `TaskStatus::Faulted` with `last_fault == fault`,
- `Operation::Rollback` authority on the fault resource,
- a policy bound to `(resource, fault_kind)`.

For `RouteToHandler`, apply additionally requires an active installed handler,
message capacity, and three event slots. It appends a `MessageKind::Fault`
message, records `MessageSent`, records `FaultRouted`, then records
`FaultPolicyApplied`.

For `RecoverTask`, apply requires two event slots. It moves the task back to
`Accepted`, records `TaskFaultRecovered`, then records `FaultPolicyApplied`.

All capacity and authority failures leave policy records, fault records,
messages, task state, and event logs unchanged.

## Test Evidence

Tests must prove:

- installing a policy records a policy and `FaultPolicyInstalled`,
- route policy application sends the fault message and records
  `FaultPolicyApplied` after `FaultRouted`,
- recover policy application moves the task back to `Accepted` and records
  `FaultPolicyApplied` after `TaskFaultRecovered`,
- installing requires rollback authority and rejects duplicate bindings,
- applying requires a policy,
- route policy message-store-full and event-log-full failures are atomic,
- recover policy event-log-full failure is atomic,
- facade syscalls expose install, apply, and policy inspection.
