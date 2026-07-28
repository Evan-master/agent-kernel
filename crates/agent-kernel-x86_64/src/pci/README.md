# Native PCI Authority

The `pci` module owns PCI discovery, reversible BAR inspection, and the
architecture-to-kernel claim boundary for x86_64 boot.

```text
0x0cf8 selector
  -> Configuration Mechanism 1
  -> segment-zero BDF scan
  -> immutable PciFunction records
  -> restored Type-0 BAR catalog
  -> exact BDF + PCI ID selection
  -> atomic Resource / Capability / Driver Endpoint claim
  -> BAR-scoped Driver Capability and Binding
  -> capability-gated Bus Master transition
  -> immutable command -> bounded native backend
```

## Modules

| Module | Responsibility |
| --- | --- |
| `types.rs` | validated BDF, register, class, function, and exact selector values |
| `config.rs` | selector/data transactions, latch probe, explicit mutation trait |
| `inventory.rs` | deterministic fixed-capacity discovery |
| `bar.rs` | typed BAR values and fixed six-slot sets |
| `bar_probe.rs` | decode-disable, sizing, restoration, and validation transaction |
| `resource_catalog.rs` | stable BDF catalog plus generic and exact claim selection |
| `claim.rs` | exact BAR-to-kernel-resource authority mapping |
| `command.rs` | verified memory-decode and Bus Master state transitions |
| `mod.rs` | public architecture boundary |

## Invariants

- Device numbers stop at 31 and function numbers stop at 7.
- Registers are DWORD aligned and remain below `0x100`.
- Every data read immediately follows its selector write.
- Every data write requires `PciConfigMutationAccess`.
- Probe restores the previous selector on every outcome.
- Function 1 through 7 are read only for a declared multifunction device.
- Inventory order is ascending bus, device, and function.
- Missing and overflowing inventories fail closed.
- BAR probing disables I/O, memory, and bus-master decode.
- Every touched BAR and original command value is verified after restoration.
- Bus Master stays clear until a DMA domain and active mapping exist.
- DMA revocation clears Bus Master before VT-d teardown.
- Reserved shapes, malformed pairs, unassigned bases, and overlapping regions
  cannot enter a claim.
- Driver selection requires exact BDF, vendor ID, device ID, and claimable BARs.

## Authority

The native BSP owns the only live configuration adapter before AP startup.
Ring-3 Agents receive `ResourceId` and `CapabilityId` values. They receive no
BDF-to-port translation, BAR coordinates, or raw configuration transaction.

One claim creates a function root plus one region Resource per assigned BAR.
Core preflights authority, capacity, ranges, endpoint overlap, and Event
capacity before appending any record. The architecture claim retains the exact
BAR index, Resource, Capability, and immutable physical Driver Endpoint.

The V23 boot profile targets QEMU `0000:00:04.0`, PCI ID `1b36:0002`, and
BAR0 as an eight-byte I/O region. Core re-admits a completed Agent through a
verified Driver image, derives only `Observe + Act`, binds the exact BAR
Resource, and produces the immutable transmit request. `PciSerialBackend`
polls the 16550 line-status register with a fixed budget before one native
`OUT`; rejected requests perform no device write.

The current physical executor is the ring-0 boot adapter. Native ring-3 Driver
Agent Call operations remain a separate milestone.

The V27 proof profile targets QEMU EDU at `0000:00:05.0`, PCI ID `1234:11e8`.
It binds the requester to a capability-authorized VT-d domain, verifies two-way
DMA, revokes the mapping, and observes the expected hardware fault.

## Deferred

- host-bridge window allocation and zero-base BAR assignment;
- bridge windows, expansion ROMs, MSI, and MSI-X;
- multi-device DMA domains and interrupt remapping;
- native ring-3 Driver Agent Call ABI;
- endpoint detach and hotplug.
