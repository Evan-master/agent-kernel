# Kernel Object Namespace V0 Design

## Purpose

Kernel Object Namespace V0 adds a native naming layer for agent-visible kernel
objects. It is not a file path, shell environment, POSIX directory, or host
filesystem adapter. It lets an agent bind a compact kernel key to a typed kernel
object reference inside a workspace resource, then resolve or rebind that name
through explicit capabilities.

## Scope

V0 provides:

- first-class `NamespaceEntryId`, `NamespaceKey`, `NamespaceObject`, and
  `NamespaceEntryRecord` types,
- fixed-capacity namespace storage owned by `KernelCore`,
- `bind_namespace_entry(agent, capability, namespace, key, object)` for creating
  a workspace-scoped binding,
- `resolve_namespace_entry(agent, capability, namespace, key)` for auditable
  lookup,
- `rebind_namespace_entry(agent, capability, entry, object)` for changing a
  binding while preserving entry identity,
- replayable `NamespaceEntryBound`, `NamespaceEntryResolved`, and
  `NamespaceEntryRebound` events,
- facade syscalls and read-only namespace entry inspection.

V0 intentionally does not provide strings, path parsing, hierarchical path
walks, mount tables, file descriptors, symbolic links, host filesystem access,
or POSIX compatibility semantics.

## Core Model

```rust
pub struct NamespaceEntryId(u64);

pub struct NamespaceKey(u64);

pub enum NamespaceObject {
    Agent(AgentId),
    Resource(ResourceId),
    Task(TaskId),
    Message(MessageId),
    MemoryCell(MemoryCellId),
}

pub struct NamespaceEntryRecord {
    pub id: NamespaceEntryId,
    pub owner: AgentId,
    pub namespace: ResourceId,
    pub capability: CapabilityId,
    pub key: NamespaceKey,
    pub object: NamespaceObject,
    pub revision: u64,
}
```

`KernelCore` gains an explicit `NAMESPACE_ENTRIES` capacity, a fixed namespace
entry array, a namespace entry length, and a deterministic `next_namespace_entry`
counter.

## Authority And Ordering

Namespace entries are scoped to resources of kind `ResourceKind::Workspace`.

Binding and rebinding require `Operation::Act` authority on the namespace
resource. Resolving requires `Operation::Observe` authority. All actor checks
use the existing active-agent boundary, so unknown, suspended, and retired
actors fail before namespace lookup or event mutation.

Every successful namespace operation appends exactly one event. Binding starts
at revision `1`. Rebinding updates the typed object, increments revision by
`1`, and records `NamespaceEntryRebound`. Resolving returns the typed object and
records `NamespaceEntryResolved`; if the event log is full, the object is not
returned because the lookup would be unaudited.

Capacity, duplicate-key, authorization, resource-kind, lookup, object-existence,
and event-log failures leave namespace entries and events unchanged.

## Test Evidence

Tests must prove:

- binding records a namespace entry and `NamespaceEntryBound`,
- resolving returns the stored object and records `NamespaceEntryResolved`,
- rebinding updates object and revision while recording `NamespaceEntryRebound`,
- non-workspace namespace resources return `ResourceKindMismatch` without an
  event,
- duplicate keys return `NamespaceEntryAlreadyExists`,
- missing bindings return `NamespaceEntryNotFound`,
- missing referenced objects return their native lookup error,
- missing authority returns `OperationDenied` without mutation,
- suspended actors are rejected before namespace entry lookup,
- store-full and event-log-full failures are atomic,
- facade syscalls expose the same behavior through `AgentKernel`.
