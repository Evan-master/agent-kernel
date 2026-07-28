# Native Intel VT-d Boundary

This directory owns the allocation-free Intel VT-d translation engine used by
the x86_64 machine layer.

```text
DMAR DRHD
  -> root table
  -> bus context table
  -> one bus + one Domain binding
  -> up to 256 requester contexts
  -> one shared 3-level hierarchy
  -> up to 512 leaves in one 2 MiB IOVA window
  -> context and IOTLB invalidation
  -> translation enable
  -> bounded fault observation
```

## Modules

| Module | Responsibility |
| --- | --- |
| `table_types.rs` | addresses, Domain ID, capacities, and typed failures |
| `tables.rs` | requester contexts and shared second-level table encoding |
| `intel_vtd.rs` | register protocol, invalidation, enable, disable, and faults |
| `mod.rs` | public architecture boundary |

## Invariants

- Every table page and translated address is 4 KiB aligned.
- Root and context entries reference physical memory owned by the BSP.
- One table set remains bound to its first segment-zero bus and Domain.
- All 256 functions on the bound bus have independent context entries.
- Live mappings share one 2 MiB IOVA window with 512 independent leaves.
- Detaching one requester preserves every other context and shared leaf.
- Removing one leaf preserves every other mapping.
- Context and leaf publication completes before MMIO invalidation commands.
- Register polling has a fixed budget and typed failure.
- Register-derived offsets remain inside the mapped 4 KiB MMIO page.
- Capability width, fault-register layout, and invalidation replies are checked.
- Table mutation precedes global context and IOTLB invalidation.
- VT-d fault records are decoded before they are cleared.
- Ring-3 Agents receive no table pointer or VT-d register mapping.

V28 keeps 2 MiB superpages, queued invalidation, multi-bus table ownership,
interrupt remapping, and non-coherent table maintenance outside this profile.
