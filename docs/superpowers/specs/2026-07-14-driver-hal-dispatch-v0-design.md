# Driver HAL Dispatch V0 Design

## Purpose

Driver HAL Dispatch V0 creates an honest boundary between an authorized native
driver command and execution by a device backend. The kernel already stores
submitted commands and terminal results, but it currently allows a caller to
move a command directly from `Submitted` to `Completed` or `Failed`. That proves
authorization and logging, not that a backend received the command.

This milestone adds an explicit dispatch state and a no_std HAL contract:

```text
DriverCommandSubmitted
    -> DriverCommandDispatched
    -> backend.execute(request)
    -> DriverCommandCompleted | DriverCommandFailed
```

The kernel owns authorization, state, and replay events. A HAL backend owns the
external side effect. The supervisor or architecture runtime connects the two
phases; the core never stores a trait object, host handle, function pointer, or
physical address.

## Scope

V0 provides:

- `DriverCommandStatus::Dispatched`,
- a fixed-width `DriverCommandRequest`,
- `dispatch_driver_command` and facade syscall exposure,
- `DriverCommandDispatched` replay events,
- terminal transitions that require prior dispatch,
- causal invocation validation at both submit and dispatch time,
- a new no_std `agent-kernel-hal` crate,
- `DriverBackend` and `DriverCommandOutcome`,
- a stateful supervisor virtual-device backend that executes requests,
- supervisor and x86_64 event labels, tests, and documentation.

V0 does not yet provide a physical endpoint registry, MMIO ranges, port ranges,
DMA, interrupt ingestion, device discovery, retries, cancellation, or backend
crash recovery policy. Those require kernel-owned endpoint identity rather than
passing raw addresses through command payloads.

## Command State

```rust
pub enum DriverCommandStatus {
    Submitted,
    Dispatched,
    Completed,
    Failed,
}
```

`Submitted` means the kernel accepted the driver's authorized intent.
`Dispatched` means the kernel emitted an immutable request that a backend may
execute. `Completed` and `Failed` are terminal reports from that backend.

No API may transition directly from `Submitted` to a terminal state. A command
may dispatch once and terminate once. Completed and failed records remain
queryable for audit.

## Dispatch Request

```rust
pub struct DriverCommandRequest {
    pub command: DriverCommandId,
    pub binding: DriverBindingId,
    pub resource: ResourceId,
    pub driver: AgentId,
    pub cause: Option<DeviceEventId>,
    pub invocation: Option<DriverInvocationId>,
    pub kind: DriverCommandKind,
    pub payload: DriverCommandPayload,
}
```

The request contains only kernel identities and fixed-width values. It carries
no capability because a backend must not make kernel authorization decisions.
It carries no raw host or hardware handle. A future endpoint registry will map
`ResourceId` to architecture-owned endpoint descriptors.

## Dispatch Validation

`dispatch_driver_command(driver, capability, command)` validates before
mutation:

1. active driver agent,
2. existing command in `Submitted`,
3. matching active binding and driver,
4. active device-like resource,
5. caller `Act` authority,
6. for a causal command, the linked Driver Invocation is still running in the
   same agent execution context,
7. one event-log slot.

Success changes the command to `Dispatched`, emits
`DriverCommandDispatched`, and returns the request. Failure leaves command state
and the event log unchanged.

## Terminal Reporting

`complete_driver_command` and `fail_driver_command` now require `Dispatched`.
They continue to validate active driver, binding, resource, `Act` authority, and
one event slot before storing a result.

A dispatched causal command may report its result after its invocation has
completed. External devices can finish asynchronously; the invocation-running
check belongs at submission and dispatch, not terminal reporting.

If a backend executes a request but terminal event recording cannot proceed,
the command remains `Dispatched`. The runtime retains the fixed-width outcome
and can retry only the kernel reporting phase without repeating the I/O.

## HAL Contract

`agent-kernel-hal` is a no_std architecture boundary that depends only on
`agent-kernel-core` types:

```rust
pub trait DriverBackend {
    fn execute(&mut self, request: DriverCommandRequest) -> DriverCommandOutcome;
}

pub enum DriverCommandOutcome {
    Completed(DriverCommandResult),
    Failed(DriverCommandResult),
}
```

The outcome is total: a backend always returns a fixed-width result that can be
recorded. Architecture errors are encoded as stable result codes by that
backend. The HAL crate does not depend on the facade, preventing a dependency
cycle and keeping authority inside the kernel.

## Supervisor Backend

The supervisor owns a virtual register device bound to one `ResourceId`. It
implements `DriverBackend` and mutates private backend state only when executing
a matching request. `Write` stores the value, `Read` returns it, `Reset` clears
it, and `Configure` records the supplied value. A resource mismatch returns a
failed outcome without changing device state.

The demonstration flow must:

1. submit a command,
2. dispatch it through the facade,
3. execute the returned request through the backend,
4. report the returned outcome through the existing terminal syscall,
5. complete the Driver Invocation.

## Events And Compatibility

New event kind:

- `DriverCommandDispatched`

The event carries the same binding, resource, cause, invocation, kind, and
payload as the submitted command.

This intentionally changes command lifecycle semantics: callers that previously
completed immediately after submission must dispatch first. Existing generic
capacities do not change. The new HAL crate adds no allocator or host runtime to
kernel crates.

## Layer Ownership

- `agent-kernel-core`: request type, command state machine, authorization, and
  events.
- `agent-kernel`: syscall-style dispatch exposure only.
- `agent-kernel-hal`: no_std backend contract and outcomes.
- `agent-supervisor`: virtual backend and host orchestration.
- `agent-kernel-x86_64`: event label now; physical endpoint execution follows
  after a kernel-owned endpoint registry exists.
