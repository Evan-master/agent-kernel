# Driver Endpoint Registry V0 Design

## Purpose

Driver Endpoint Registry V0 gives the kernel ownership of the mapping from a
native device-like `ResourceId` to the architecture endpoint that can execute
its commands. Agents continue to name resources and submit typed commands; they
do not provide MMIO addresses or I/O ports in command payloads.

The dispatch path becomes:

```text
Agent command -> authorized ResourceId -> registered endpoint -> HAL backend
```

This milestone follows Driver HAL Dispatch V0. It creates the configuration
boundary needed before an x86_64 backend can safely perform physical I/O.

## Scope

V0 provides:

- fixed-width endpoint kinds and descriptors,
- a fixed-capacity endpoint record store in `agent-kernel-core`,
- one immutable endpoint mapping per device-like resource,
- descriptor range validation and overlap rejection,
- explicit `Delegate` authorization for endpoint registration,
- a `DriverEndpointRegistered` audit event,
- command dispatch rejection when no endpoint exists,
- syscall facade and supervisor integration,
- virtual, MMIO, and I/O-port descriptor support,
- tests, trace labels, no_std checks, and QEMU regression coverage.

V0 does not perform volatile MMIO or port instructions. It does not discover
PCI devices, map virtual memory, allocate DMA, route interrupts, detach or
replace endpoints, or expose endpoint registration to an unprivileged Agent.

## Endpoint Identity

The endpoint identity is the existing `ResourceId`. This avoids a second
identity that could diverge from the resource, binding, command, and capability
stores. A command request already carries this identity in its `resource`
field, so Driver HAL Dispatch does not need a compatibility-breaking request
shape change.

The registry resolves that resource to a descriptor only inside trusted kernel
or architecture orchestration. A command payload never carries the descriptor.

## Data Model

```rust
pub enum DriverEndpointKind {
    Virtual,
    Mmio,
    Port,
}

pub struct DriverEndpointDescriptor {
    pub kind: DriverEndpointKind,
    pub base: u64,
    pub span: u64,
}

pub struct DriverEndpointRecord {
    pub resource: ResourceId,
    pub installer: AgentId,
    pub descriptor: DriverEndpointDescriptor,
}
```

Convenience constructors create virtual channels, MMIO ranges, and port
ranges. Registration validates every descriptor even when its fields were
constructed directly.

The endpoint store uses `RESOURCES` as its fixed capacity. Because V0 permits
at most one endpoint per existing resource, it cannot require more slots than
the resource store and does not add another const-generic parameter to every
kernel module.

## Registration

`register_driver_endpoint(installer, capability, resource, descriptor)` checks:

1. the installer is active,
2. the resource exists and is active,
3. the resource kind is `Device`, `Network`, or `Service`,
4. the capability authorizes `Delegate` on that resource,
5. no endpoint is already registered for the resource,
6. the descriptor has a nonzero, non-overflowing span,
7. port ranges end at or below `0xffff`,
8. the range does not overlap another endpoint of the same kind,
9. one event slot is available.

Success appends the immutable record and emits `DriverEndpointRegistered`.
Failure changes neither the store nor the event log.

Numeric ranges in different endpoint kinds are separate address spaces and may
overlap. V0 keeps descriptors reserved even after their resources retire. Safe
reuse requires an explicit detach lifecycle and is intentionally deferred.

## Dispatch Integration

`dispatch_driver_command` resolves the command resource in the endpoint store
before changing command state or returning a HAL request. A missing endpoint
returns `DriverEndpointNotFound` and leaves the command `Submitted`.

Submission does not require an endpoint. This permits boot or supervisor code
to stage authorized work before architecture initialization completes, while
the external side-effect boundary remains closed until endpoint registration.
Driver binding and device-event delivery are also independent of endpoint
registration.

## Events And Inspection

`DriverEndpointRegistered` records installer, resource, capability, and
`Delegate` operation. The full descriptor remains queryable through
`driver_endpoints()` and the facade.

The generic event shape does not duplicate physical coordinates. This matches
the existing resource event model and prevents serial or supervisor traces from
leaking MMIO and port locations by default.

## Layer Ownership

- `agent-kernel-core`: endpoint values, validation, fixed store, event, and
  dispatch gate.
- `agent-kernel`: syscall-style registration and read-only record exposure.
- `agent-kernel-hal`: unchanged request/outcome contract; `ResourceId` remains
  the endpoint identity.
- `agent-supervisor`: registers a virtual endpoint and constructs its backend
  from the resulting kernel record.
- `agent-kernel-x86_64`: event label only in V0; volatile physical access is the
  next architecture milestone.
