# Fault Handler Routing V0 Design

## Purpose

Fault Handler Routing V0 turns a recorded task fault into a kernel-routable
agent message. Task Fault Trap V0 records that a running task trapped; this V0
adds a deterministic handler table so an authorized agent can install a handler
for `(resource, fault_kind)` and route a fault to that handler without host
callbacks, POSIX signals, dynamic dispatch, or supervisor-only convention.

## Scope

V0 provides:

- first-class `FaultHandlerId` and `FaultHandlerRecord` types,
- fixed-capacity fault handler storage owned by `KernelCore`,
- `install_fault_handler(agent, capability, resource, kind, handler_agent)`,
- `route_fault_to_handler(agent, capability, fault)`,
- `FaultHandlerInstalled` and `FaultRouted` events,
- `MessageKind::Fault` and `MessagePayload::fault`,
- facade syscalls, supervisor output, QEMU labels, and documentation.

V0 intentionally does not provide automatic trap vector execution inside
`fault_task`, retry policies, handler priority chains, wildcard resource
handlers, handler task spawning, automatic rollback execution, dynamic payload
buffers, or model calls.

## Core Model

```rust
pub struct FaultHandlerId(u64);

pub struct FaultHandlerRecord {
    pub id: FaultHandlerId,
    pub resource: ResourceId,
    pub kind: FaultKind,
    pub installer: AgentId,
    pub handler: AgentId,
}
```

`MessageKind` gains `Fault`, and `MessagePayload` gains
`fault: Option<FaultId>`.

## Authority And Ordering

Installing a handler requires:

- active installer agent,
- active handler agent,
- `Operation::Rollback` authority on the resource,
- unused `(resource, fault_kind)` binding,
- handler store capacity,
- one event slot.

Routing a fault requires:

- active routing agent,
- an existing `FaultRecord`,
- the task still in `TaskStatus::Faulted` with `last_fault == fault`,
- `Operation::Rollback` authority on the fault resource,
- an active registered handler for `(resource, fault_kind)`,
- message store capacity,
- two event slots.

A successful route appends one pending `MessageKind::Fault` message to the
handler with resource, task, and fault IDs in the payload. It records
`MessageSent` first and then `FaultRouted`, so replay sees the delivered
message before the routing audit event.

Handler install duplicate, missing handler, stale recovered fault,
message-store-full, event-log-full, inactive-agent, and missing-authority
failures leave handler records, messages, fault records, task state, and event
logs unchanged.

## Test Evidence

Tests must prove:

- installing a handler records the handler and `FaultHandlerInstalled`,
- installing requires rollback authority and rejects duplicates atomically,
- routing a fault sends a pending fault message to the handler,
- routing records `MessageSent` before `FaultRouted`,
- the routed message payload includes resource, task, and fault IDs,
- routing requires a registered handler,
- routing rejects stale recovered faults,
- message-store-full and event-log-full route failures are atomic,
- facade syscalls expose install, route, and handler inspection.
