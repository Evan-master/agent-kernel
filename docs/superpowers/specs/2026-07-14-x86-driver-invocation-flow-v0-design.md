# X86 Driver Invocation Flow V0 Design

## Purpose

The current x86 QEMU path proves that a kernel-dispatched command can reach a
native Port backend and return a terminal result. That command is cause-free,
so the physical proof still bypasses the kernel's device-event and Driver
Invocation runtime.

This milestone closes that gap with a polled physical flow:

```text
Driver Agent reads COM1 line status through a kernel command
  -> bootstrap raises a DeviceEvent from the physical result
  -> kernel delivers the event and queues a Driver Invocation
  -> kernel dispatches and ticks the Driver Invocation
  -> Driver acknowledges the event
  -> Driver submits a causal COM1 write command
  -> native Port backend executes the immutable request
  -> kernel completes the command and Driver Invocation
```

This is deliberately described as polling, not an interrupt. IDT, PIC/APIC,
interrupt stubs, and asynchronous hardware ingress remain separate work.

## Architecture Placement

The kernel core and facade already own all required records, authorization, and
transitions. This milestone does not add a parallel runtime or a privileged
shortcut.

- `agent-kernel-boot` proves that its opt-in Driver store capacities can carry
  the causal lifecycle after bootstrap.
- `agent-kernel-x86_64` orchestrates the physical polling adapter and owns the
  unsafe native Port authority.
- `agent-kernel-core` remains the sole owner of event, invocation, command, and
  Agent execution-context state.
- `agent-kernel-hal` remains an immutable request/result boundary.

The x86 boot type opts into one binding, one device event, two commands, one
Driver Invocation, and enough event slots for the complete trace.

## Physical Poll

After endpoint registration, Driver admission, image verification, launch, and
binding, the Driver submits a cause-free byte-wide `Read` command at relative
offset five, the COM1 line-status register. The request is dispatched through
the endpoint gate and executed by `PortIoBackend`.

The read outcome must be terminally recorded before its value is used. A
successful result with the transmitter-empty bit set becomes a
`DeviceEventKind::StateChanged` payload. The payload code is the relative
line-status offset and the payload value is the returned register byte.

Architecture code does not construct either command request and does not read
the line-status port behind the kernel command path for this event.

## Driver Invocation

The bootstrap Agent raises the event under its existing `Act` authority. The
Driver Agent receives attenuated `Observe` and `Act` authority, then:

1. delivers the event and atomically queues its Driver Invocation,
2. dispatches the invocation with quantum two,
3. records one explicit tick,
4. acknowledges the event,
5. submits a `Write` command with the event as its cause,
6. dispatches a request whose `invocation` field identifies the running
   invocation.

The native backend writes the marker byte at relative offset zero. The outcome
is recorded through the matching terminal command syscall. A successful path
then completes the Driver Invocation, returns the Driver Agent execution context
to idle, and verifies the event, command, and invocation records.

## QEMU Proof

The existing markers remain:

```text
AGENT_KERNEL_PORT_IO_BACKEND_OK
AGENT_KERNEL_PORT_COMMAND_FLOW_OK
```

After the causal invocation reaches its terminal record, QEMU also emits:

```text
AGENT_KERNEL_DRIVER_INVOCATION_FLOW_OK
```

The event trace must include both polling-command transitions followed by
device-event raise/delivery, invocation queue/dispatch/tick, acknowledgement,
causal write-command transitions, and invocation completion.

## Failure Semantics

A rejected setup or dispatch exits before unapproved physical I/O. A backend
failure is always recorded through `sys_fail_driver_command`. A failed poll,
missing transmitter-ready bit, causal identity mismatch, terminal transition
error, or record mismatch fails the QEMU proof. Architecture code never edits
kernel records directly.

## Deferred Work

V0 still executes trusted orchestration rather than instructions loaded from the
Driver image. It does not install an IDT, route a UART IRQ, program PIC/APIC,
handle nested interrupts, preempt an Agent, map MMIO, discover PCI, or authorize
DMA. The next physical-ingress milestone can replace polling with a real x86
interrupt while preserving this event and invocation contract.
