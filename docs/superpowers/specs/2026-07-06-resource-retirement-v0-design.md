# Resource Retirement V0 Design

## Purpose

Resource Retirement V0 gives kernel resources an explicit lifecycle. Today a
resource exists forever once allocated. V0 adds an `Active` / `Retired` state so
an authorized agent can remove a resource from future capability use without
deleting audit history or reusing IDs.

## Scope

V0 provides:

- first-class `ResourceStatus`,
- `retire_resource(agent, capability, resource)`,
- `ResourceRetired` event,
- `ResourceRetired` error,
- public resource inspection through `resources()`,
- facade syscall, supervisor output, tests, and documentation.

V0 intentionally does not provide recursive child retirement, dependency
analysis, resource deletion, ID reuse, garbage collection, cancellation of
tasks that already reference the resource, or host filesystem/device teardown.

## Core Model

```rust
pub enum ResourceStatus {
    Active,
    Retired,
}

pub struct Resource {
    pub id: ResourceId,
    pub kind: ResourceKind,
    pub parent: Option<ResourceId>,
    pub status: ResourceStatus,
}
```

## Authority And Ordering

Retiring a resource requires:

- active acting agent,
- root `Operation::Rollback` authority on the resource,
- active resource,
- one event slot.

`retire_resource` marks the resource `Retired` and records `ResourceRetired`.

After retirement:

- `find_resource` rejects the resource with `KernelError::ResourceRetired`,
- new capability grants on it fail,
- operations authorized by old capabilities fail,
- child resource registration under it fails.

All authorization and capacity failures leave resource state and event logs
unchanged.

## Test Evidence

Tests must prove:

- retiring records `ResourceRetired` and changes the resource status,
- retiring requires rollback authority without mutation,
- event-log-full leaves the resource active,
- retired resources reject future grants, old-capability operations, and child
  registration,
- facade syscall exposes retirement and resource inspection.
