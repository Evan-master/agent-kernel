# X86 Port Command Flow V0 Design

## Purpose

X86 Port Command Flow V0 connects the existing kernel Driver command state
machine to the native x86 Port backend. The previous milestone proved that a
kernel-registered endpoint could drive a real `out` instruction, but its QEMU
probe constructed `DriverCommandRequest` directly in architecture code. This
milestone removes that shortcut.

The proven path becomes:

```text
Driver Agent authority
  -> kernel command Submitted
  -> kernel endpoint-gated Dispatched request
  -> bounded x86 Port backend
  -> kernel Completed or Failed terminal event
```

## Boot Capacity Extension

`BootedKernel` currently fixes all Driver stores at their zero-valued
`AgentKernel` defaults. It gains four trailing const generics with zero
defaults:

- `DRIVER_BINDINGS`,
- `DEVICE_EVENTS`,
- `DRIVER_COMMANDS`,
- `DRIVER_INVOCATIONS`.

Existing ten-argument users remain unchanged. The boot crate still fixes
unrelated host-style stores to zero and keeps Agent Image capacity equal to
Agent capacity. The x86 boot type opts into one binding and one command; it does
not allocate device-event or invocation capacity because the V0 proof submits a
cause-free command.

The x86 entry explicitly requests a 256 KiB boot stack. Constructing the
fixed-capacity kernel by value in an unoptimized bare-metal build exceeds the
bootloader's default 80 KiB stack and reaches its guard page. The larger bounded
stack preserves guard-page detection while making the QEMU debug path reliable.

## Physical Flow

After the deterministic bootstrap flow, trusted x86 initialization:

1. registers the COM1 `Port` endpoint for the bootstrap Device resource,
2. registers a dedicated Driver Agent,
3. derives `Act` authority for that resource,
4. registers and verifies a Driver image,
5. launches the Driver Agent into a resource-scoped Driver entry,
6. binds it to the bootstrap Device resource,
7. submits one byte-wide `Write` command with relative offset zero,
8. dispatches the command through the kernel endpoint gate,
9. executes the returned immutable request through `PortIoBackend`,
10. reports the backend result through `sys_complete_driver_command` or
    `sys_fail_driver_command`.

The command ID, binding ID, resource, driver, kind, and payload all come from
the kernel dispatch request. Architecture code no longer invents request
identity.

## QEMU Proof

The dispatched command writes the `O` byte in:

```text
AGENT_KERNEL_PORT_IO_BACKEND_OK
```

After the terminal kernel transition succeeds, the boot entry emits:

```text
AGENT_KERNEL_PORT_COMMAND_FLOW_OK
```

The event log then includes the endpoint, Driver Agent, derived capability,
Driver image, launch, binding, submit, dispatch, and completion events. The
terminal command record must contain `Completed` and the backend result.

## Failure Semantics

Initialization failures exit QEMU through the error path before physical I/O.
Once a request is dispatched, a backend `Failed` outcome is recorded through
the kernel failure syscall before the probe reports failure. A completion or
failure transition error also fails the boot probe. No architecture code edits
kernel records directly.

## Deferred Work

V0 still uses trusted boot orchestration instead of executing instructions from
the Driver Agent image. It does not add a device event, Driver Invocation,
interrupt, MMIO, DMA, endpoint replacement, or general device protocol. Those
are separate milestones after the command side-effect chain is closed.
