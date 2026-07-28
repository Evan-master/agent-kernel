# Native Intel VT-d Boundary

This directory owns the allocation-free Intel VT-d translation engine used by
the x86_64 machine layer.

```text
DMAR DRHD
  -> root table
  -> bus context table
  -> domain + requester binding
  -> 3-level second-level page tables
  -> context and IOTLB invalidation
  -> translation enable
  -> bounded fault observation
```

## Modules

| Module | Responsibility |
| --- | --- |
| `tables.rs` | root, context, and second-level table encoding |
| `intel_vtd.rs` | register protocol, invalidation, enable, disable, and faults |
| `mod.rs` | public architecture boundary |

## Invariants

- Every table page and translated address is 4 KiB aligned.
- Root and context entries reference physical memory owned by the BSP.
- One table set remains bound to its first Requester and Domain.
- Register polling has a fixed budget and typed failure.
- Register-derived offsets remain inside the mapped 4 KiB MMIO page.
- Capability width, fault-register layout, and invalidation replies are checked.
- Mapping removal precedes context and IOTLB invalidation.
- VT-d fault records are decoded before they are cleared.
- Ring-3 Agents receive no table pointer or VT-d register mapping.

V27 deliberately supports one requester, one domain, and one 4 KiB mapping.
Multi-device domains, superpages, queued invalidation, and interrupt remapping
remain future milestones.
