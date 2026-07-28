# Native DMA/IOMMU V27 Design

## Goal

V27 gives Agent Kernel its first native DMA authority boundary and proves that
boundary against an emulated PCI DMA engine and Intel VT-d:

```text
Agent Capability graph
        |
        +-- IOMMU Resource
        +-- DMA Domain Resource
        +-- PCI Function Resource
        +-- Memory Resource
                    |
                    +-- reserved DMA mapping
                    +-- VT-d translation installed
                    +-- mapping activated
                    +-- PCI bus mastering enabled
                                      |
                                      +-- QEMU EDU DMA succeeds

mapping revocation
        |
        +-- Core enters revoking state
        +-- VT-d leaf removed and invalidated
        +-- Core records release
        +-- repeated EDU DMA raises a VT-d fault
        +-- protected destination bytes remain unchanged
```

The normal QEMU profile remains unchanged. A dedicated
`qemu-dma-iommu-proof` profile uses the Q35 machine, one Intel IOMMU, and one
QEMU EDU PCI function.

## Core Model

`agent-kernel-core` owns architecture-neutral, fixed-capacity records:

| Record | Identity | Authority link | Lifecycle |
| --- | --- | --- | --- |
| DMA domain | `ResourceId` | IOMMU Resource | active |
| DMA attachment | device Resource | domain Capability + device Capability | attached |
| DMA mapping | `DmaMappingId` | domain Capability + memory Capability | reserved, active, revoking, released, cancelled |

DMA domains use `ResourceKind::DmaDomain`. Hardware remapping units use
`ResourceKind::Iommu`. PCI functions continue to use `ResourceKind::Device`;
DMA buffers use `ResourceKind::Memory`.

Core stores use existing `RESOURCES` and `CAPS` bounds. They add no allocator,
host I/O, architecture address, or PCI-specific structure.

## Mapping Contract

A mapping request contains:

- one DMA domain Resource;
- one Memory Resource;
- a page-aligned I/O virtual address;
- a nonzero bounded page count;
- read, write, or read-write device access.

Reservation validates both Capabilities, resource kinds, address arithmetic,
store capacity, and overlap within the domain. Activation is valid only from
the reserved state.

Revocation is two-phase:

1. `begin_dma_unmap` changes active to revoking and denies further use.
2. the architecture owner removes hardware translation and invalidates caches.
3. `complete_dma_unmap` changes revoking to released.

A failed hardware installation can cancel a reserved mapping. Every state
transition appends a deterministic Event.

## Architecture Boundary

`agent-kernel-x86_64` owns:

- strict ACPI DMAR discovery and checksum validation;
- DRHD selection for one PCI requester;
- legacy VT-d root and context entries;
- three-level, 39-bit second-stage translation tables;
- MMIO register programming and bounded status polling;
- context-cache and IOTLB invalidation;
- fault-record decoding and clearing;
- QEMU EDU register access.

Physical addresses never enter `agent-kernel-core`. The architecture owner
binds an active Core mapping to one exclusive physical frame while holding the
corresponding semantic records.

## Native Proof Profile

The proof topology is fixed:

| Component | Configuration |
| --- | --- |
| machine | Q35 |
| IOMMU | `intel-iommu`, 39-bit address width |
| DMA function | QEMU EDU, `1234:11e8`, BDF `00:05.0` |
| EDU BAR | BAR0, 1 MiB MMIO |
| domain | VT-d domain ID 1 |
| mapped IOVA | `0x0100_0000` |
| transfer | 64 bytes |

The profile runs through a dedicated boot entry before SMP startup and legacy
PCI INTx setup. This keeps Q35-specific routing outside the existing default
proof.

## Activation Order

The boot proof follows this sequence:

1. discover and validate DMAR;
2. discover EDU and probe BAR0;
3. clear EDU memory-space and bus-master command bits;
4. create IOMMU, device, memory, and DMA-domain Resources;
5. attach the EDU requester through Capabilities;
6. reserve a Core mapping;
7. allocate and zero exclusive translation/data frames;
8. install VT-d tables with translation disabled;
9. set the root pointer, invalidate caches, and enable translation;
10. activate the Core mapping;
11. enable EDU memory-space and bus-master command bits;
12. execute a RAM-to-EDU-to-RAM round trip and compare bytes.

Any failure before step 11 leaves PCI bus mastering disabled.

## Revocation Proof

After the permitted round trip:

1. Core moves the mapping to revoking;
2. the VT-d leaf entry is cleared;
3. context and IOTLB caches are invalidated;
4. Core completes the release;
5. EDU attempts the same device-to-memory transfer;
6. VT-d reports the EDU source ID, unmapped IOVA, and denied write;
7. the destination frame retains its poison pattern;
8. the fault record is cleared and observed clear.

The proof succeeds only when both allowed and blocked outcomes are observed.

## Failure Semantics

- malformed, duplicate, oversized, or checksum-invalid DMAR data fails closed;
- an unsupported host address width or missing 39-bit capability fails closed;
- a DRHD that does not cover EDU fails closed;
- overlapping Core mappings leave state and Events unchanged;
- reserved, active, and revoking mappings keep their IOVA ranges occupied;
- mapped domains, IOMMUs, devices, and memory reject resource retirement;
- retired IOMMUs and attached devices reject new DMA work;
- invalid lifecycle transitions leave state and Events unchanged;
- register-derived VT-d offsets outside the mapped page fail closed;
- incompatible address width, fault layout, or invalidation replies fail closed;
- a table set cannot be rebound to another requester or domain;
- VT-d status timeouts halt before bus mastering;
- failures after Bus Master activation attempt verified quiescence before halt;
- a fault during the permitted transfer fails the proof;
- a missing fault during the revoked transfer fails the proof;
- any destination mutation after revocation fails the proof.

## Verification

V27 requires:

- Core DMA lifecycle, authorization, overlap, capacity, and event-order tests;
- DMAR parser and discovery tests;
- VT-d table encoding and register-sequence tests;
- EDU register contract tests;
- debug and release QEMU DMA/IOMMU proofs;
- default debug and release QEMU regressions;
- workspace tests, strict Clippy, formatting, Supervisor replay, shell syntax,
  package audit, and bare-target checks.

## Validated Evidence

| Gate | Result |
| --- | --- |
| Core, facade, and machine contracts | `cargo test --workspace` |
| Host lint | workspace, all targets, `-D warnings` |
| Bare lint | default and `qemu-dma-iommu-proof`, `-D warnings` |
| Default QEMU | debug + release, status `33`, Supervisor handoff |
| V27 QEMU | debug + release, status `33`, terminal proof marker |
| Allowed transfer | RAM to EDU to RAM, exact 64-byte pattern restored |
| Revoked transfer | source `0x28`, IOVA `0x0100_0000`, denied write, reason `5` |
| Protected memory | poison pattern unchanged after the denied device write |
| Repository tooling | format, Supervisor replay, shell/Ruby syntax, image audit, Go build |

The expected QEMU diagnostic reports a second-level permission failure after
the leaf entry is cleared. The kernel accepts the proof only after decoding
the matching VT-d fault record and observing unchanged destination bytes.

## References

- [QEMU EDU device specification](https://www.qemu.org/docs/master/specs/edu.html)
- [QEMU Intel IOMMU invocation reference](https://www.qemu.org/docs/master/system/qemu-manpage.html)
- [QEMU 11.0.2 EDU implementation](https://gitlab.com/qemu-project/qemu/-/blob/v11.0.2/hw/misc/edu.c)
- [QEMU 11.0.2 Intel IOMMU implementation](https://gitlab.com/qemu-project/qemu/-/blob/v11.0.2/hw/i386/intel_iommu.c)

## Exclusions

V27 leaves MSI/MSI-X, interrupt remapping, queued invalidation, ATS, PASID,
scalable-mode translation, huge DMA pages, scatter/gather lists, multiple PCI
segments, and production device assignment for later milestones.
