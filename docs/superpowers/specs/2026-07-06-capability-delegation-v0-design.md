# Capability Delegation V0 Design

## Purpose

Capability Delegation V0 lets an agent derive a narrower root capability for
another active agent from authority it already holds. The kernel already derives
task-scoped capabilities during task delegation, but there is no general
agent-to-agent authority handoff. This feature makes authority sharing explicit,
auditable, and bounded by parent capability revocation.

## Scope

V0 provides:

- `derive_capability(actor, source_capability, target_agent, operations)`,
- facade syscall `sys_derive_capability`,
- operation-set subset validation,
- `CapabilityDerived` event records for general delegation,
- parent-chain revocation semantics through the existing capability chain,
- supervisor and README coverage.

V0 intentionally does not remove bootstrap `grant_capability`, introduce
resource ownership, support multi-parent capabilities, add time leases, or
change task-scoped delegation behavior.

## Core Model

General delegation creates a normal root capability:

```rust
Capability {
    agent: target_agent,
    resource: source.resource,
    operations,
    revoked: false,
    task: None,
    parent: Some(source_capability),
}
```

The event reuses `EventKind::CapabilityDerived`:

- `agent`: actor that delegated authority,
- `target_agent`: receiving agent,
- `resource`: source capability resource,
- `capability`: new derived capability,
- `source_capability`: parent capability,
- `operations`: delegated operation set,
- `task`: `None`.

## Authority Rules

Deriving a capability requires:

- active actor,
- active target agent,
- active resource,
- source capability belongs to the actor,
- source capability is not task-scoped,
- source capability grants `Operation::Delegate`,
- requested operations are a subset of source operations,
- parent capability chain is not revoked,
- one capability slot,
- one event slot.

If any validation fails, no capability is allocated and no event is recorded.

## After Delegation

The target agent can use the derived capability for any delegated operation.
Revoking the derived capability blocks only that derived capability. Revoking
the source capability blocks all derived descendants through the existing
capability-chain authorization check.

## Test Evidence

Tests must prove:

- general delegation records `CapabilityDerived` and target agent can use it,
- delegation requires `Delegate` authority,
- delegation cannot expand operations beyond the source capability,
- task-scoped capabilities cannot become root capabilities,
- event-log-full leaves no derived capability,
- source revocation invalidates derived root capabilities,
- facade syscall exposes the same behavior.
