# Driver Command V0 Design

## Purpose

Driver Command V0 completes the first bidirectional native driver contract in
Agent Kernel. Driver bindings and device events already model input from an
external resource to its bound Driver Agent. This design adds the output side:
the bound Driver Agent can submit a typed command to that resource and move the
command to a terminal success or failure result.

The resulting replayable flow is:

```text
DeviceEventRaised -> DeviceEventDelivered -> DeviceEventAcknowledged
                                               |
                                               v
DriverCommandSubmitted -> DriverCommandCompleted | DriverCommandFailed
```

The command can carry an optional source `DeviceEventId`. This creates a
kernel-owned causal link between an external event and the driver's response.
The kernel does not perform physical I/O in this milestone; a future HAL or
device executor will consume submitted records through a separate boundary.

## Scope

V0 provides:

- `DriverCommandId`, `DriverCommandKind`, `DriverCommandPayload`,
  `DriverCommandResult`, `DriverCommandStatus`, and `DriverCommandRecord`,
- a fixed-capacity command store owned by `KernelCore`,
- `submit_driver_command`, `complete_driver_command`, and
  `fail_driver_command`,
- optional causal linkage to a delivered or acknowledged device event,
- explicit binding and `Operation::Act` authorization checks,
- replayable submit, complete, and fail events,
- syscall facade methods and read-only inspection,
- a supervisor trace that exercises event-to-command causality.

V0 does not provide byte buffers, DMA, physical register access, interrupt
controller programming, command cancellation, priorities, retries, deadlines,
automatic dispatch, POSIX I/O, or a Linux driver compatibility layer.

## Core Model

```rust
pub enum DriverCommandKind {
    Configure,
    Read,
    Write,
    Reset,
}

pub struct DriverCommandPayload {
    pub opcode: u16,
    pub value: u64,
}

pub struct DriverCommandResult {
    pub code: u16,
    pub value: u64,
}

pub enum DriverCommandStatus {
    Submitted,
    Completed,
    Failed,
}

pub struct DriverCommandRecord {
    pub id: DriverCommandId,
    pub binding: DriverBindingId,
    pub resource: ResourceId,
    pub driver: AgentId,
    pub cause: Option<DeviceEventId>,
    pub kind: DriverCommandKind,
    pub payload: DriverCommandPayload,
    pub status: DriverCommandStatus,
    pub result: Option<DriverCommandResult>,
}
```

All values are fixed-width and copyable. Payloads are not pointers, host
handles, paths, socket addresses, or allocator-backed buffers.

## Storage

`KernelCore` and `AgentKernel` gain a trailing capacity parameter:

```rust
const DRIVER_COMMANDS: usize = 0,
```

The core owns:

```rust
driver_commands: [DriverCommandRecord; DRIVER_COMMANDS],
driver_command_len: usize,
next_driver_command: u64,
```

The independent capacity prevents command pressure from consuming device-event
storage. The trailing default preserves older kernel instantiations.

## Submission Contract

```rust
submit_driver_command(
    driver: AgentId,
    capability: CapabilityId,
    resource: ResourceId,
    cause: Option<DeviceEventId>,
    kind: DriverCommandKind,
    payload: DriverCommandPayload,
) -> Result<DriverCommandId, KernelError>
```

Validation completes before mutation:

1. the driver agent is active,
2. the resource exists, is active, and is `Device`, `Network`, or `Service`,
3. a driver binding exists for the resource,
4. the binding names the caller as its driver,
5. the capability authorizes `Operation::Act` for the resource,
6. an optional cause exists, belongs to the same binding and resource, and is
   already `Delivered` or `Acknowledged`,
7. the command store has capacity,
8. one event slot is available.

Success allocates an ID, stores a `Submitted` record with no result, and emits
`DriverCommandSubmitted`. Any failure leaves the ID allocator, command store,
and event log unchanged.

## Terminal Transition Contract

```rust
complete_driver_command(driver, capability, command, result)
fail_driver_command(driver, capability, command, result)
```

Both operations require an active bound driver, an active target resource,
`Operation::Act`, a currently `Submitted` command, and one free event slot.
They atomically set the terminal status and result, then emit either
`DriverCommandCompleted` or `DriverCommandFailed`. A terminal command cannot be
completed or failed again.

## Events And Errors

New event kinds:

- `DriverCommandSubmitted`
- `DriverCommandCompleted`
- `DriverCommandFailed`

Events include the command ID, kind, payload, result, binding, resource,
capability, and optional source device event.

New errors:

- `DriverCommandStoreFull`
- `DriverCommandNotFound`
- `DriverCommandStatusMismatch`
- `DriverCommandCauseMismatch`

Existing errors continue to describe inactive agents, retired resources,
missing bindings, wrong drivers, denied operations, missing device events, and
full event logs.

## Boundary Rules

- `agent-kernel-core` owns deterministic records, transitions, authorization,
  and event logging only.
- `agent-kernel` exposes syscall-style methods without bypassing core checks.
- `agent-supervisor` simulates an executor result but cannot mutate records.
- Physical I/O and host adapters remain outside both no_std crates.
- No command path grants capabilities implicitly.
