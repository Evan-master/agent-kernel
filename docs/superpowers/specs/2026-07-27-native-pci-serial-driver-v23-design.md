# Native PCI Serial Driver V23 Design

**Status:** implemented and verified

## Objective

V23 connects the V22 PCI Resource claim to one physical controller command.
The boot owner selects an exact QEMU PCI serial function, re-admits a completed
Agent as the dedicated controller Driver, delegates the claimed BAR Region, and
executes one authorized 16550 transmit request through native x86 port I/O.

```text
PCI selector
  -> claimed BAR0 Region
  -> Driver Capability
  -> Driver Binding
  -> Device Event / Invocation
  -> immutable Driver Command
  -> 16550 backend
  -> x86 OUT
  -> QEMU chardev byte
```

## Hardware Target

The proof machine adds the documented QEMU single-port PCI serial controller:

```text
BDF        0000:00:04.0
PCI ID     1b36:0002
BAR0       I/O / 8 bytes
UART       16550A
```

References:

- [QEMU PCI serial devices](https://www.qemu.org/docs/master/specs/pci-serial.html)
- [QEMU PCI IDs](https://www.qemu.org/docs/master/specs/pci-ids.html)

The QEMU command pins the device to slot 4. Discovery still reads the identity
and BAR from configuration space; no address or BAR value is fabricated by the
kernel.

## Exact Target Selection

The architecture layer gains a copyable `PciFunctionSelector`:

```rust
pub struct PciFunctionSelector {
    address: PciFunctionAddress,
    vendor_id: u16,
    device_id: u16,
}
```

Catalog selection requires all three fields to match and requires the selected
Function to produce a valid Driver Resource Tree specification. Stable first
candidate selection remains available for generic inventory inspection, but
the V23 boot path uses only the exact selector.

The retained `PciFunctionClaim` must match this selector during installation
and before Driver setup.

## Driver Authority

The V22 Region Capability remains owned by the bootstrap Agent and includes
`Delegate`. V23 uses `AgentId 10` after its completed Worker entry has been
retired. The Agent stays active, has no live entry, and receives a new Driver
entry scoped only to BAR0.

The fixed Agent Image store is full after the V22 slot-reuse proof. V23 retires
the unreferenced pending `AgentImageId 15`, retires its record, and installs a
verified Driver image in the recovered slot. Agent and image capacities remain
14, so the proof exercises real lifecycle reuse.

V23 derives one Capability for the re-admitted Driver Agent:

```text
resource     claimed BAR0 Region
operations   Observe + Act
source       bootstrap-owned Region Capability
target       Driver Agent
```

The bootstrap Agent then binds that Driver Agent to the BAR Region with the
owner Capability. The backend is constructed from the immutable endpoint
record returned by Core. No BDF, port base, or Capability is supplied by the
Driver command payload.

## 16550 Backend

`PciSerialBackend<I: PortIo>` implements `DriverBackend`.

Construction requires:

- a `Port` endpoint;
- an endpoint span of at least 8 bytes;
- a base whose final register remains within `u16`;
- a nonzero transmit poll budget.

The V23 command contract is:

```text
resource    exact claimed BAR0 Region
kind        Write
opcode      0 / transmit holding register
value       one byte
```

Execution polls Line Status Register offset 5 for THRE bit `0x20`. The backend
writes the byte to offset 0 only after readiness. Resource mismatch, unsupported
kind or opcode, oversized values, and timeout produce typed failure results and
no transmit write.

## Event Contract

V22 terminates at Event 417. V23 appends:

```text
418  AgentImageRetired
419  AgentImageRecordRetired
420  CapabilityDerived
421  AgentImageRegistered
422  AgentImageVerified
423  AgentLaunched
424  DriverBound
425  DeviceEventRaised
426  DeviceEventDelivered
427  DriverInvocationQueued
428  DriverInvocationDispatched
429  DriverInvocationTicked
430  DeviceEventAcknowledged
431  DriverCommandSubmitted
432  DriverCommandDispatched
433  DriverCommandCompleted
434  DriverInvocationCompleted
```

Driver Binding, Device Event, Driver Command, and Driver Invocation capacities
increase from one to two. The PCI Driver phase reserves 17 Events. Existing IDs
and the V22 Event prefix remain stable.

## Physical Side-Effect Proof

QEMU receives a dedicated file-backed chardev for the PCI serial controller.
The authorized command transmits byte `0x50` (`P`). `run-qemu.sh` requires:

- all V23 boot markers;
- exact Events `1..434`;
- QEMU debug-exit status 33;
- a one-byte PCI serial output file;
- exact output byte `0x50`.

This proves the command crossed the kernel authority boundary and reached the
emulated physical controller.

## Failure Semantics

- Missing or mismatched PCI target stops boot before claim installation.
- A non-I/O or non-eight-byte BAR cannot construct the serial backend.
- Re-admission requires an active Agent with no live Agent Entry.
- Image replacement requires an unreferenced pending image and authorized
  retirement of both image state and record.
- Capability derivation and Driver Binding complete before any port access.
- Backend validation failures perform no port I/O.
- Poll timeout performs reads only and emits no transmit write.
- Failed hardware execution records a failed Driver Command and stops handoff.
- Ring-3 receives no raw BDF or port coordinate.

## Deferred Work

- native ring-3 Driver Agent Call operations;
- PCI INTx routing for the added controller;
- MMIO Driver backends;
- MSI and MSI-X;
- DMA/IOMMU domains;
- controller detach and hotplug.

## Verification

V23 requires:

- selector construction and exact-match tests;
- wrong-address and wrong-identity fail-closed tests;
- serial backend constructor and request-validation tests;
- bounded-ready and bounded-timeout transport tests;
- Core authority and terminal-record assertions;
- exact QEMU chardev side-effect validation;
- workspace tests, strict host and bare-metal Clippy, image audits, QEMU debug,
  and QEMU release.
