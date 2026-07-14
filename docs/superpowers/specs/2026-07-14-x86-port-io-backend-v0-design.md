# X86 Port I/O Backend V0 Design

## Purpose

X86 Port I/O Backend V0 turns a kernel-owned `Port` endpoint into a real x86
I/O execution boundary. Agents still submit typed commands against a
`ResourceId`; they never submit an absolute port address. The architecture
backend resolves a relative command offset inside the endpoint and performs the
physical instruction only after all resource, descriptor, range, command, and
value checks pass.

The path becomes:

```text
Agent command -> kernel authorization -> endpoint lookup -> bounded port -> x86 in/out
```

## Layer Ownership

- `agent-kernel-core` owns resources, capabilities, endpoint records, command
  state, and dispatch authorization. It remains architecture-neutral.
- `agent-kernel-hal` owns the existing backend request/outcome contract.
- `agent-kernel-x86_64` owns port descriptor interpretation and x86 `in`/`out`
  instructions.
- `agent-kernel-boot` exposes mutable handoff access so trusted architecture
  initialization can register the endpoint through the normal kernel facade.

No raw port instruction or address interpretation enters the core or facade
crates.

## Endpoint Contract

`PortIoBackend` is constructed from a `DriverEndpointRecord` and a `PortIo`
implementation. Construction validates the record again even though normal
records came from the kernel registry:

1. endpoint kind is `Port`,
2. span is nonzero,
3. `base + span - 1` does not overflow,
4. the inclusive end is at or below `0xffff`.

The backend stores the endpoint resource, base, and span. It cannot execute a
request naming another resource.

## Command Semantics

V0 supports byte-wide operations only:

- `Read`: `payload.opcode` is a relative port offset; one `in` byte is returned
  in `DriverCommandResult.value`.
- `Write`: `payload.opcode` is a relative port offset; `payload.value` must fit
  in `u8`, and one `out` byte is performed. The written byte is echoed in the
  result.
- `Configure` and `Reset`: rejected as unsupported without I/O. Device-specific
  multi-register protocols need a later typed command schema, not implicit
  architecture behavior.

The computed offset must be less than the endpoint span. This keeps Agent data
relative to an authorized resource and prevents it from becoming an absolute
hardware coordinate.

## Outcome Codes

The backend returns fixed-width outcomes:

| Code | Meaning |
| ---: | --- |
| 0 | completed |
| 1 | request resource mismatch |
| 2 | relative offset outside endpoint span |
| 3 | write value does not fit in one byte |
| 4 | command kind unsupported by V0 |

All failure outcomes are side-effect free.

## Testability And Native I/O

The backend depends on a small `PortIo` trait. Host tests provide a recording
implementation and verify exact ports, values, read results, and absence of I/O
on every rejected request. The native implementation is available only on
`x86_64` and uses inline assembly.

Creating native port authority is `unsafe`: the caller must already be running
at an x86 privilege level where the requested port operation is legal and must
only pair it with a trusted kernel endpoint. Once created, the backend performs
all per-request bounds checks through a safe `DriverBackend` interface.

## Bare-Metal Proof

The x86 boot path grants the bootstrap Agent `Delegate` authority, registers a
COM1 `Port` endpoint through `AgentKernel`, resolves the resulting immutable
record, and constructs the native backend from it. A bounded write at offset
zero emits the middle byte of:

```text
AGENT_KERNEL_PORT_IO_BACKEND_OK
```

over QEMU serial. The prefix and suffix use the existing serial path; the `O`
byte is emitted by `PortIoBackend`. QEMU output therefore proves that endpoint
registration, architecture validation, and the native `out` instruction all
execute on the bare-metal target.

## Deferred Work

V0 does not provide 16/32-bit port widths, MMIO, interrupt routing, PCI
discovery, DMA, endpoint replacement, concurrent backend ownership, or a fully
booted Driver Agent command lifecycle. Those remain separate milestones so the
physical side-effect boundary stays inspectable.
