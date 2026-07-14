# Driver Runtime Dispatch V0 Design

## Purpose

Driver Runtime Dispatch V0 turns native driver records into schedulable Agent
work. The kernel already knows which agent is bound to a device-like resource,
can deliver typed device events, and can record commands sent back by the
driver. Before this milestone, the supervisor can acknowledge an event and
submit a command without the Driver Agent ever entering a kernel-owned running
state.

This design adds a first-class `DriverInvocation`. Event delivery atomically
creates queued work for the bound Driver Agent. The kernel then dispatches that
work with a deterministic quantum, owns the Agent execution-context transition,
and requires the invocation to be running before the event can be acknowledged
or used as the cause of a command.

```text
DeviceEventRaised
    -> DeviceEventDelivered + DriverInvocationQueued
    -> DriverInvocationDispatched
    -> DeviceEventAcknowledged
    -> DriverCommandSubmitted -> DriverCommandCompleted | DriverCommandFailed
    -> DriverInvocationCompleted
```

This is an AgentOS-native runtime path. It does not create a process, thread,
file descriptor, signal handler, or Linux-compatible driver callback.

## Scope

V0 provides:

- `DriverInvocationId`, `DriverInvocationStatus`, and
  `DriverInvocationRecord`,
- `AgentImageKind::Driver` and `AgentEntryKind::Driver`,
- a fixed-capacity invocation store that also preserves FIFO queue order,
- atomic event delivery and invocation queueing,
- driver-specific runtime admission from a verified image and launch entry,
- `dispatch_next_driver_invocation`, `tick_driver_invocation`, and
  `complete_driver_invocation`,
- task/driver mutual exclusion in `AgentExecutionContext`,
- running-invocation checks for acknowledgement and causally linked commands,
- replayable invocation queue, dispatch, tick, quantum expiry, and completion
  events,
- facade syscalls, supervisor trace coverage, and x86_64 event labels.

V0 does not provide physical device execution, preemptive timer interrupts,
invocation faults, invocation cancellation, priority scheduling, SMP, DMA,
allocator-backed payloads, or legacy driver compatibility.

## Runtime Types

```rust
pub struct DriverInvocationId(u64);

pub enum DriverInvocationStatus {
    Queued,
    Running,
    Completed,
}

pub struct DriverInvocationRecord {
    pub id: DriverInvocationId,
    pub binding: DriverBindingId,
    pub driver: AgentId,
    pub resource: ResourceId,
    pub event: DeviceEventId,
    pub status: DriverInvocationStatus,
    pub run_ticks: u64,
    pub quantum_remaining: u64,
}
```

The invocation store is append-only in V0. The first record with `Queued`
status for a given driver is that driver's next runnable invocation. Completed
records remain queryable for audit and deterministic replay.

`AgentExecutionContext` gains:

```rust
pub driver_invocation: Option<DriverInvocationId>
```

The kernel invariant is that an execution context can reference either a task
or a driver invocation, never both. Existing `Running`, `Idle`, `Waiting`, and
`Faulted` states remain; driver invocations use only `Running` and `Idle` in V0.

## Driver Image And Entry

Add `Driver` variants to `AgentImageKind` and `AgentEntryKind`. A Driver image
must be verified for the same resource as its launch entry. The launch entry's
capability must authorize `Operation::Act`; runtime admission additionally
requires that the entry:

- belongs to the bound driver agent,
- is `AgentEntryKind::Driver`,
- is scoped to the invocation resource,
- still has active `Observe` and `Act` authority for that resource.

Binding a driver does not launch it and still does not grant capabilities.
Delivery refuses to queue work for an unlaunched or incorrectly scoped driver.

## Storage

`KernelCore` and `AgentKernel` gain one trailing capacity:

```rust
const DRIVER_INVOCATIONS: usize = 0,
```

The core owns:

```rust
driver_invocations: [DriverInvocationRecord; DRIVER_INVOCATIONS],
driver_invocation_len: usize,
next_driver_invocation: u64,
```

No second queue allocation is needed. Append order plus status filtering is a
deterministic per-driver FIFO queue.

## Delivery And Queueing

`deliver_device_event(driver, capability, event)` now returns a
`DriverInvocationId` and performs one atomic transition.

Before mutation it validates:

1. active driver,
2. existing `Raised` event and active resource,
3. matching driver binding,
4. caller `Observe` authority,
5. valid Driver launch admission,
6. invocation-store capacity,
7. two event-log slots.

Success changes the event to `Delivered`, allocates a queued invocation, emits
`DeviceEventDelivered`, then emits `DriverInvocationQueued`. Any failure leaves
the event, invocation store, ID allocator, execution context, and log unchanged.

## Dispatch And Quantum

```rust
dispatch_next_driver_invocation(driver, quantum) -> DriverInvocationId
tick_driver_invocation(driver, invocation) -> Event
```

Dispatch requires a non-zero quantum, an active admitted driver, a queued
invocation, an active resource, an idle execution context, and one event slot.
It chooses the oldest queued invocation for that driver, marks it `Running`,
sets the context's `driver_invocation`, and emits
`DriverInvocationDispatched`.

Each explicit tick increments `run_ticks`. A non-final tick emits
`DriverInvocationTicked`. The final tick clears the execution context, returns
the invocation to `Queued`, and emits `DriverInvocationQuantumExpired`. Store
order makes redispatch deterministic.

## Acknowledgement, Commands, And Completion

An event can be acknowledged only while its invocation is `Running` for the
bound driver. A command with that event as its cause also requires the same
running invocation; the resulting `DriverCommandRecord` and events store the
invocation ID as an additional causal link.

```rust
complete_driver_invocation(driver, capability, invocation) -> Event
```

Completion requires:

- active admitted bound driver,
- invocation status `Running`,
- matching running execution context,
- active resource and `Act` authority,
- associated event status `Acknowledged`,
- one event slot.

Success marks the invocation `Completed`, clears the execution context, and
emits `DriverInvocationCompleted`.

## Events And Errors

New event kinds:

- `DriverInvocationQueued`
- `DriverInvocationDispatched`
- `DriverInvocationTicked`
- `DriverInvocationQuantumExpired`
- `DriverInvocationCompleted`

Events carry invocation ID, binding, device event, resource, ticks, and quantum
where applicable.

New errors:

- `DriverInvocationStoreFull`
- `DriverInvocationNotFound`
- `DriverInvocationStatusMismatch`
- `DriverInvocationQueueEmpty`
- `DriverInvocationQuantumInvalid`
- `DriverInvocationNotRunnable`
- `AgentEntryKindMismatch`

## Boundaries

- `agent-kernel-core` owns records, admission, queue order, execution state,
  authorization, and replay events.
- `agent-kernel` only exposes syscall-style calls and read-only records.
- `agent-supervisor` supplies explicit ticks and simulates driver behavior.
- A future architecture/HAL layer will provide interrupts and execute physical
  commands; no host handles enter the core or facade.
