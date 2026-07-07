# Agent Driver Binding V0 Design

## Purpose

Agent Driver Binding V0 gives Agent Kernel its first native driver boundary.
The current kernel can classify resources as `Device`, `Network`, or `Service`,
but those resources do not yet have a kernel-visible way to name the agent that
is responsible for handling external events from them. That leaves device
control as a supervisor convention instead of a replayable kernel fact.

This V0 adds deterministic driver bindings and device event lifecycle records:

```text
DriverBound
DeviceEventRaised -> DeviceEventDelivered -> DeviceEventAcknowledged
```

The kernel still does not perform hardware I/O. It records authority,
responsibility, event state, and audit events so a supervisor or future HAL can
feed external events into an agent-native kernel contract without defining the
core around Linux drivers, files, sockets, or processes.

## Scope

V0 provides:

- first-class `DriverBindingId` and `DeviceEventId` types,
- `DriverBindingRecord` for one active driver agent per device-like resource,
- `DeviceEventRecord` for fixed-width external event facts,
- `DeviceEventKind`, `DeviceEventPayload`, and `DeviceEventStatus`,
- fixed-capacity driver binding and device event stores owned by `KernelCore`,
- `bind_driver(installer, capability, resource, driver_agent)`,
- `raise_device_event(agent, capability, resource, kind, payload)`,
- `deliver_device_event(driver, capability, event)`,
- `acknowledge_device_event(driver, capability, event)`,
- `DriverBound`, `DeviceEventRaised`, `DeviceEventDelivered`, and
  `DeviceEventAcknowledged` event kinds,
- facade syscalls and read-only inspection,
- supervisor output and README trace updates.

V0 does not provide:

- real hardware access,
- interrupt controller programming,
- DMA,
- byte buffers,
- host filesystem or network I/O,
- Linux driver compatibility,
- automatic capability grants to drivers,
- driver replacement or unbinding,
- event priority queues,
- broadcast delivery,
- timers,
- model calls.

The supervisor may simulate a device event source. Kernel crates only store and
authorize deterministic records.

## Resource Model

Driver binding is allowed only for resource kinds that represent externally
controlled system parts:

```rust
ResourceKind::Device
ResourceKind::Network
ResourceKind::Service
```

Binding a driver to `Workspace`, `Memory`, `File`, or `Process` returns the
existing `KernelError::ResourceKindMismatch` and leaves all stores unchanged.
This keeps V0 focused on the native driver boundary instead of turning namespace
objects, memory cells, or legacy classifications into driver endpoints.

Retired resources reject binding, event raising, delivery, and acknowledgement
through the existing active-resource lookup path. Existing records remain
queryable for audit.

## Core Types

Add typed IDs:

```rust
pub struct DriverBindingId(u64);
pub struct DeviceEventId(u64);
```

Add driver binding records:

```rust
pub struct DriverBindingRecord {
    pub id: DriverBindingId,
    pub installer: AgentId,
    pub resource: ResourceId,
    pub resource_kind: ResourceKind,
    pub driver: AgentId,
}
```

V0 allows exactly one binding per resource. A duplicate binding returns
`KernelError::DriverBindingAlreadyExists`.

Add device event records:

```rust
pub enum DeviceEventKind {
    Interrupt,
    DataReady,
    Fault,
    StateChanged,
}

pub struct DeviceEventPayload {
    pub code: u16,
    pub value: u64,
}

pub enum DeviceEventStatus {
    Raised,
    Delivered,
    Acknowledged,
}

pub struct DeviceEventRecord {
    pub id: DeviceEventId,
    pub binding: DriverBindingId,
    pub resource: ResourceId,
    pub kind: DeviceEventKind,
    pub payload: DeviceEventPayload,
    pub status: DeviceEventStatus,
}
```

The payload is deliberately fixed-width. It is not a byte buffer, packet, file
path, socket address, pointer, or host handle.

## KernelCore Storage

Add trailing capacity parameters to `KernelCore`:

```rust
const DRIVER_BINDINGS: usize = 0,
const DEVICE_EVENTS: usize = 0,
```

Add fields:

```rust
driver_bindings: [DriverBindingRecord; DRIVER_BINDINGS],
device_events: [DeviceEventRecord; DEVICE_EVENTS],
driver_binding_len: usize,
device_event_len: usize,
next_driver_binding: u64,
next_device_event: u64,
```

Defaults keep old instantiations source-compatible because the new capacities
are trailing const generics with defaults.

## Binding Contract

Add:

```rust
bind_driver(
    installer: AgentId,
    capability: CapabilityId,
    resource: ResourceId,
    driver_agent: AgentId,
) -> Result<DriverBindingId, KernelError>
```

Validation happens before mutation:

- installer is active,
- driver agent is active,
- resource exists and is active,
- resource kind is `Device`, `Network`, or `Service`,
- capability authorizes `Operation::Delegate` for the resource,
- no binding already exists for the resource,
- driver binding store has capacity,
- one event slot is available.

On success, the kernel:

- allocates `DriverBindingId` from `next_driver_binding`,
- stores the binding,
- records `EventKind::DriverBound`,
- returns the binding ID.

Binding does not grant the driver any capability. The installer or supervisor
must explicitly grant or derive any `Observe` or `Act` authority the driver will
use later.

## Device Event Raising Contract

Add:

```rust
raise_device_event(
    agent: AgentId,
    capability: CapabilityId,
    resource: ResourceId,
    kind: DeviceEventKind,
    payload: DeviceEventPayload,
) -> Result<DeviceEventId, KernelError>
```

Validation happens before mutation:

- agent is active,
- resource exists and is active,
- resource kind is `Device`, `Network`, or `Service`,
- capability authorizes `Operation::Act` for the resource,
- an active driver binding exists for the resource,
- device event store has capacity,
- one event slot is available.

On success, the kernel:

- allocates `DeviceEventId` from `next_device_event`,
- stores a `DeviceEventRecord` with status `Raised`,
- records `EventKind::DeviceEventRaised`,
- returns the device event ID.

Failure leaves `next_device_event`, device event records, and the event log
unchanged.

## Device Event Delivery Contract

Add:

```rust
deliver_device_event(
    driver: AgentId,
    capability: CapabilityId,
    event: DeviceEventId,
) -> Result<Event, KernelError>
```

Validation happens before mutation:

- driver is active,
- event exists,
- event status is `Raised`,
- event binding exists,
- binding driver equals `driver`,
- resource still exists and is active,
- capability authorizes `Operation::Observe` for the event resource,
- one event slot is available.

On success, the kernel:

- changes event status to `Delivered`,
- records `EventKind::DeviceEventDelivered`,
- returns the event.

Delivery is a kernel-visible handoff to the driver agent. It does not enqueue a
mailbox message in V0; the event store itself is the deterministic queueable
record surface.

## Device Event Acknowledgement Contract

Add:

```rust
acknowledge_device_event(
    driver: AgentId,
    capability: CapabilityId,
    event: DeviceEventId,
) -> Result<Event, KernelError>
```

Validation happens before mutation:

- driver is active,
- event exists,
- event status is `Delivered`,
- event binding exists,
- binding driver equals `driver`,
- resource still exists and is active,
- capability authorizes `Operation::Act` for the event resource,
- one event slot is available.

On success, the kernel:

- changes event status to `Acknowledged`,
- records `EventKind::DeviceEventAcknowledged`,
- returns the event.

Acknowledgement means the driver agent has accepted responsibility for handling
the device event. It is not a verified outcome and does not replace action or
task verification.

## Event Model

Add event kinds:

```rust
DriverBound
DeviceEventRaised
DeviceEventDelivered
DeviceEventAcknowledged
```

Extend `Event` with:

```rust
driver_binding: Option<DriverBindingId>,
device_event: Option<DeviceEventId>,
device_event_kind: Option<DeviceEventKind>,
device_event_payload: Option<DeviceEventPayload>,
```

`DriverBound` stores:

- `agent` as the installer,
- `resource`,
- `capability`,
- `driver_binding`,
- `target_agent` as the driver.

`DeviceEventRaised` stores:

- raising `agent`,
- `resource`,
- `capability`,
- `driver_binding`,
- `device_event`,
- `device_event_kind`,
- `device_event_payload`.

`DeviceEventDelivered` and `DeviceEventAcknowledged` store:

- driver `agent`,
- `resource`,
- `capability`,
- `driver_binding`,
- `device_event`,
- `device_event_kind`,
- `device_event_payload`.

All mutating paths either record the corresponding event or leave state
unchanged with an explicit error.

## Error Model

Add:

```rust
DriverBindingStoreFull
DriverBindingNotFound
DriverBindingAlreadyExists
DeviceEventStoreFull
DeviceEventNotFound
DeviceEventStatusMismatch
```

Use existing errors where they already express the failure:

- `AgentNotFound`, `AgentSuspended`, `AgentRetired`,
- `ResourceNotFound`, `ResourceRetired`, `ResourceKindMismatch`,
- `CapabilityNotFound`, `CapabilityRevoked`, `CapabilityScopeMismatch`,
- `AgentMismatch`, `OperationDenied`,
- `EventLogFull`.

Driver mismatch uses `KernelError::AgentMismatch` because the requested driver
agent is not the driver recorded in the binding.

## Facade Contract

`agent-kernel` exposes syscall-style wrappers:

```rust
sys_bind_driver(...)
sys_raise_device_event(...)
sys_deliver_device_event(...)
sys_acknowledge_device_event(...)
driver_binding(id)
device_event(id)
```

The facade does not add policy. It forwards to `KernelCore` and exposes
read-only record inspection.

## Supervisor Flow

The supervisor demonstration adds a small native device flow after the existing
resource and namespace operations:

1. owner creates a `Device` resource under the workspace,
2. owner derives or grants explicit driver authority to the target agent,
3. owner binds the target agent as the device driver,
4. owner raises a `DeviceEventKind::StateChanged` event,
5. target agent delivers the event with observe authority,
6. target agent acknowledges the event with act authority.

Expected trace additions:

```text
driver_bound
device_event_raised
device_event_delivered
device_event_acknowledged
```

The supervisor still simulates the event source. No host device access moves
into kernel crates.

## Tests

Core tests must prove:

- binding a driver records a `DriverBindingRecord` and `DriverBound`,
- binding requires `Operation::Delegate`,
- binding rejects inactive driver agents without allocation,
- binding rejects non-device-like resource kinds with `ResourceKindMismatch`,
- duplicate binding leaves the original binding and event log unchanged,
- binding store full leaves the event log unchanged,
- event log full leaves the binding store unchanged,
- raising an event stores `Raised` status and records `DeviceEventRaised`,
- raising requires `Operation::Act`,
- raising requires an existing binding,
- raising store full and event log full are atomic,
- delivery requires the bound driver and `Operation::Observe`,
- delivery moves `Raised` to `Delivered` and records
  `DeviceEventDelivered`,
- acknowledgement requires the bound driver and `Operation::Act`,
- acknowledgement moves `Delivered` to `Acknowledged` and records
  `DeviceEventAcknowledged`,
- repeated delivery or acknowledgement returns `DeviceEventStatusMismatch`
  without mutation,
- retired resources reject event transitions without mutation.

Facade tests must prove syscall wrappers expose the same lifecycle and record
inspection. Supervisor tests must assert the four new labels appear in order.

## no_std And Determinism

All new core and facade code remains no_std-compatible. Records are fixed-size
copyable values. Stores are fixed-capacity arrays. There is no allocation,
filesystem access, networking, threads, timers, randomness, environment access,
or host I/O in `agent-kernel-core` or `agent-kernel`.

The kernel records device event facts; it does not execute drivers, call host
adapters, parse byte streams, or inspect hardware.

## Compatibility Impact

The public `KernelCore` type gains trailing const generics with defaults, so
existing type aliases and tests keep compiling until they opt into driver
binding and device event capacity.

Existing resource, capability, task, image, supervisor, and QEMU behavior should
remain unchanged until the supervisor demonstration explicitly exercises the new
driver lifecycle.

## Follow-Up

After V0, the next native OS step should be one of:

- driver unbinding and replacement with explicit authority,
- event wait integration so driver tasks can sleep on device events,
- HAL adapter traits outside kernel core,
- quota/budget enforcement for event-heavy drivers.
