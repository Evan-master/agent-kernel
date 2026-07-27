# Native PCI Inventory

The `pci` module owns read-only PCI function discovery for the x86_64 boot
boundary.

```text
0x0cf8 selector
  -> Configuration Mechanism 1
  -> segment-zero BDF scan
  -> immutable PciFunction records
  -> BSP-owned PciInventory
```

## Modules

| Module | Responsibility |
| --- | --- |
| `types.rs` | validated BDF, register, class, and function values |
| `config.rs` | exclusive selector/data transaction and latch probe |
| `inventory.rs` | deterministic fixed-capacity discovery |
| `mod.rs` | public architecture boundary |

## Invariants

- Device numbers stop at 31 and function numbers stop at 7.
- Registers are DWORD aligned and remain below `0x100`.
- Every data read immediately follows its selector write.
- Probe restores the previous selector on every outcome.
- Function 1 through 7 are read only for a declared multifunction device.
- Inventory order is ascending bus, device, and function.
- Missing and overflowing inventories fail closed.
- Configuration-data writes are unavailable.

## Authority

The native BSP owns the only live configuration adapter before AP startup.
Ring-3 Agents receive no BDF-to-port translation and no raw configuration
transaction.

The next device-authority layer will claim one inventoried function, size and
reserve its BARs under a reversible transaction, create a kernel `Resource`,
and bind a least-authority Driver endpoint.
