# Capability Revocation Propagation V0 Design

## Purpose

Capability Revocation Propagation V0 makes delegated task authority depend on
the source capability that created it. The current delegated capability model
derives a task-scoped capability during `delegate_task`, but that derived
capability remains usable even after the delegating source capability is
revoked. That leaves revocation incomplete: the owner can revoke direct
authority, but previously delegated task authority survives.

This design makes derived capabilities remember their parent capability and
requires authorization to reject any capability whose parent chain contains a
revoked capability.

## Selected Approach

Add `parent: Option<CapabilityId>` to `Capability` and check the parent chain
during authorization.

Alternatives considered:

- Eagerly revoke all derived children when a source capability is revoked:
  deterministic, but it requires scanning and mutating the whole capability
  store for every revocation.
- Add a separate delegation graph: explicit, but it duplicates capability store
  relationships before the kernel needs graph-level introspection.
- Keep revocation local to the exact capability id: simple, but it weakens the
  authority boundary created by delegated task capabilities.

Parent-chain validation is the right V0 because it keeps revocation propagation
inside the authorization path, preserves fixed-capacity no_std storage, and does
not need heap allocation or a graph subsystem.

## Architecture Placement

`agent-kernel-core` owns:

- capability parent metadata,
- parent assignment during derived task capability allocation,
- bounded parent-chain revocation checks during authorization,
- task delegation passing the source capability into derivation.

`agent-kernel` owns:

- no syscall shape changes in V0,
- compiling against the extended public `Capability` shape.

`agent-supervisor` owns:

- no behavior changes in V0.

Boot and image crates stay unchanged in behavior. They compile with the updated
capability shape but do not derive delegated task capabilities during boot.

## Data Model

Extend `Capability`:

```rust
pub struct Capability {
    pub id: CapabilityId,
    pub agent: AgentId,
    pub resource: ResourceId,
    pub operations: OperationSet,
    pub revoked: bool,
    pub task: Option<TaskId>,
    pub parent: Option<CapabilityId>,
}
```

`parent: None` means the capability is a root grant.

`parent: Some(source_capability)` means the capability was derived from another
capability and is valid only while every capability in that parent chain remains
not revoked.

The existing `task` field keeps its current meaning:

- `task: None` is normal resource authority,
- `task: Some(task_id)` is authority scoped to one task.

## Operation Semantics

`grant_capability`:

- keeps its public signature,
- creates root capabilities with `task: None` and `parent: None`.

`derive_task_capability`:

- remains internal to `agent-kernel-core`,
- accepts the source capability id as a parent,
- creates task-scoped capabilities with `task: Some(task)` and
  `parent: Some(source_capability)`.

`delegate_task(agent, capability, task, target_agent)`:

- keeps the existing source capability authorization checks,
- passes the source capability id into `derive_task_capability`,
- keeps recording `DelegationRequested` with the derived capability id.

`revoke_capability(capability)`:

- keeps setting only the directly targeted capability's `revoked` flag,
- does not scan or mutate derived children in V0,
- remains intentionally invisible in the event log in V0 because the current
  event model does not yet include capability lifecycle events. This is a
  documented gap; a future capability lifecycle event should close it.

Authorization:

- rejects the direct capability if it is revoked,
- walks `parent` links until a root capability is reached,
- rejects with `CapabilityRevoked` if any parent is revoked,
- returns `CapabilityNotFound` if a parent id is missing,
- bounds traversal to at most `CAPS` parent hops so corrupted internal state
  cannot produce an infinite loop,
- still applies existing agent, resource, operation, and task-scope checks to
  the direct capability being used by the caller.

Parent capabilities do not need to match the current caller, operation, or task.
Their role in V0 is source validity, not re-authorizing the whole operation.

## Error Handling

Use existing errors:

- `CapabilityRevoked` when the direct capability or any parent capability is
  revoked,
- `CapabilityNotFound` when a parent id cannot be found or a bounded traversal
  cannot reach a root capability,
- `AgentMismatch`, `ResourceMismatch`, `OperationDenied`, and
  `CapabilityScopeMismatch` for the existing direct capability checks.

Failed authorization must leave task state, capability state, and event log
unchanged.

## Determinism And Authority

Parent assignment is deterministic: the source capability id is the exact
capability used to authorize `delegate_task`. Parent-chain checks use only the
fixed-capacity capability store. There is no host I/O, model call, allocation,
randomness, or wall-clock input.

This design tightens least authority. Revoking the source of delegated authority
invalidates all task capabilities derived from it without granting the kernel a
hidden background mutation mechanism.

## Tests

Implementation must start with failing tests.

Core tests:

- revoking the source capability invalidates the derived task capability before
  task completion,
- revoking the derived capability itself still invalidates task completion,
- revoking an unrelated capability does not invalidate the derived task
  capability,
- one source revocation invalidates multiple derived task capabilities.

Existing tests should continue to prove:

- revoking a normal capability prevents generic authorization,
- task-scoped capabilities cannot authorize generic resource operations,
- task-scoped capabilities cannot complete a different task.

Full verification:

```bash
PATH="$HOME/.cargo/bin:$PATH" rustup run nightly cargo fmt --check
PATH="$HOME/.cargo/bin:$PATH" RUSTC="$(rustup which rustc --toolchain nightly)" rustup run nightly cargo test --workspace
PATH="$HOME/.cargo/bin:$PATH" RUSTC="$(rustup which rustc --toolchain nightly)" rustup run nightly cargo run -p agent-supervisor
scripts/run-qemu.sh
```

## Compatibility Impact

This design changes the public `Capability` struct shape by adding
`parent: Option<CapabilityId>`. Internal initializers and docs must be updated.

No POSIX, Linux syscall, shell, or legacy compatibility model is added.

## Deferred Work

V0 does not include:

- explicit capability lifecycle events,
- querying child capabilities by parent,
- capability attenuation beyond the existing task-scoped action derivation,
- parent-chain replay from event logs,
- leases, expiration, or policy-time constraints.
