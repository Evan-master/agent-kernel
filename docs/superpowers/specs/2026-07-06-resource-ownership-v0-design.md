# Resource Ownership V0 Design

## Purpose

Resource Ownership V0 gives kernel-created resources an explicit owning agent.
The current kernel can register resources and then separately grant root
capabilities. That is useful for bootstrap, but it keeps resource authority too
detached from the agent that creates the resource. V0 adds an owner-aware
creation path that allocates a resource and its first capability atomically.

## Scope

V0 provides:

- `Resource.owner: Option<AgentId>`,
- `ResourceCreateOutcome { resource, capability }`,
- `create_resource(agent, kind, parent, operations)`,
- facade syscall `sys_create_resource`,
- `ResourceCreated` event,
- supervisor and README coverage.

V0 keeps `register_resource(kind, parent)` for bootstrap and system-seeded
resources. Those resources have `owner: None`. V0 does not remove direct
`grant_capability`, implement owner transfer, recursive ownership, resource
quotas, or garbage collection.

## Core Model

```rust
pub struct Resource {
    pub id: ResourceId,
    pub kind: ResourceKind,
    pub parent: Option<ResourceId>,
    pub owner: Option<AgentId>,
    pub status: ResourceStatus,
}

pub struct ResourceCreateOutcome {
    pub resource: ResourceId,
    pub capability: CapabilityId,
}
```

`create_resource` takes:

```rust
create_resource(
    agent: AgentId,
    kind: ResourceKind,
    parent: Option<(ResourceId, CapabilityId)>,
    operations: OperationSet,
) -> Result<ResourceCreateOutcome, KernelError>
```

For a root resource, `parent` is `None`. For a child resource, `parent` carries
the parent resource ID and a capability that must authorize `Operation::Act` on
that parent resource.

## Event Model

Successful creation records two events in order:

1. `ResourceCreated`
2. `CapabilityGranted`

Both events reference the new resource and the initial capability. The
capability event uses the requested operation set. The resource-created event
also stores the operation set so replay can inspect the initial authority that
was created with the resource.

## Atomicity And Authority

Before mutating state, `create_resource` validates:

- active creating agent,
- active parent resource if one is provided,
- `Operation::Act` authority on the parent if one is provided,
- one resource slot,
- one capability slot,
- two event slots.

If any validation fails, no resource is allocated, no capability is allocated,
and no event is recorded.

## Test Evidence

Tests must prove:

- root resource creation sets `owner: Some(agent)`,
- creation records `ResourceCreated` then `CapabilityGranted`,
- returned capability can authorize the requested operation,
- child resource creation requires act authority on the parent,
- inactive agents, full stores, full event log, and retired parents leave no
  partial resource/capability/event state,
- facade syscall exposes the same behavior.
