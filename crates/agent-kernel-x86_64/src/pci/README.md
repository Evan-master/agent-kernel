# Native PCI Authority

The `pci` module owns PCI discovery, reversible BAR inspection, and the
architecture-to-kernel claim boundary for x86_64 boot.

```text
0x0cf8 selector
  -> Configuration Mechanism 1
  -> segment-zero BDF scan
  -> immutable PciFunction records
  -> restored Type-0 BAR catalog
  -> atomic Resource / Capability / Driver Endpoint claim
```

## Modules

| Module | Responsibility |
| --- | --- |
| `types.rs` | validated BDF, register, class, and function values |
| `config.rs` | selector/data transactions, latch probe, explicit mutation trait |
| `inventory.rs` | deterministic fixed-capacity discovery |
| `bar.rs` | typed BAR values and fixed six-slot sets |
| `bar_probe.rs` | decode-disable, sizing, restoration, and validation transaction |
| `resource_catalog.rs` | stable BDF catalog and deterministic claim candidate |
| `claim.rs` | exact BAR-to-kernel-resource authority mapping |
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
- Reserved shapes, malformed pairs, unassigned bases, and overlapping regions
  cannot enter a claim.

## Authority

The native BSP owns the only live configuration adapter before AP startup.
Ring-3 Agents receive `ResourceId` and `CapabilityId` values. They receive no
BDF-to-port translation, BAR coordinates, or raw configuration transaction.

One claim creates a function root plus one region Resource per assigned BAR.
Core preflights authority, capacity, ranges, endpoint overlap, and Event
capacity before appending any record. The architecture claim retains the exact
BAR index, Resource, Capability, and immutable physical Driver Endpoint.

## Deferred

- host-bridge window allocation and zero-base BAR assignment;
- bridge windows, expansion ROMs, MSI, and MSI-X;
- DMA/IOMMU domains and bus-master enable;
- endpoint detach and controller-specific Driver execution.
