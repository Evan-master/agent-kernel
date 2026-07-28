# Native MSI/MSI-X and Multi-Device DMA V28 Design

## Goal

V28 extends the native device authority boundary in three connected areas:

```text
Device Capability
        |
        +-- Interrupt Route Resource
        |       +-- MSI -> xAPIC vector 0xd0
        |       +-- MSI-X entry 0 -> xAPIC vector 0xd1
        |
        +-- shared DMA Domain
                +-- EDU requester 00:05.0
                +-- virtio-rng requester 00:06.0
                +-- fixed-capacity IOVA mappings

hardware activation
        |
        +-- VT-d contexts installed for both requesters
        +-- EDU DMA completion raises MSI
        +-- virtio entropy completion raises MSI-X
        +-- both handlers execute through the native IDT and Local APIC

requester detachment
        |
        +-- virtio-rng is quiesced
        +-- Core enters detaching state
        +-- its VT-d context is removed and invalidated
        +-- a denied probe produces a requester-specific VT-d fault
        +-- EDU continues operating in the same DMA Domain
```

The default profile and V27 profile remain independently runnable. A dedicated
`qemu-msi-msix-proof` profile owns the Q35 topology used by V28.

## Core Interrupt Model

`agent-kernel-core` adds architecture-neutral interrupt records:

| Record | Identity | Authority | Lifecycle |
| --- | --- | --- | --- |
| Interrupt route | `ResourceId` | parent Device Capability | reserved, active, revoking, released |
| Target | destination ID + vector | route Capability | immutable |
| Mode | MSI or MSI-X entry | route Capability | immutable |

Interrupt routes use `ResourceKind::InterruptRoute`. Route creation produces a
child Resource and an initial Capability. The route store contains no APIC
address, PCI offset, MMIO pointer, or architecture register value.

The route request contains:

- one active Device Resource;
- one Capability authorized for `Act` on that Device;
- MSI or MSI-X mode;
- one MSI-X table entry when the mode is MSI-X;
- a destination identifier;
- one interrupt vector in the architectural device-vector range;
- operations for the resulting route Capability.

Core rejects duplicate route keys and duplicate live destination/vector pairs.
MSI accepts entry zero only. Every state transition appends a deterministic
Event.

Revocation is two-phase:

1. `begin_interrupt_route_revoke` changes active to revoking.
2. the architecture owner masks or disables the hardware source.
3. `complete_interrupt_route_revoke` changes revoking to released.

Device retirement remains unavailable while one of its routes is reserved,
active, or revoking.

## Multi-Device DMA Model

DMA attachments gain an explicit lifecycle:

| State | Meaning |
| --- | --- |
| attached | requester may use active mappings in the Domain |
| detaching | Core authority is closed; hardware context removal is pending |
| detached | requester no longer occupies the Device or requester identity |

Attachment and detachment require both the Domain Capability and Device
Capability. Detachment follows this sequence:

1. quiesce the PCI function and mask its interrupt routes;
2. move the Core attachment to detaching;
3. clear the requester context entry;
4. invalidate the VT-d context cache and IOTLB;
5. complete the Core detach transition.

Mappings remain Domain-scoped. Other attached requesters retain access after
one requester is detached. New mappings require at least one attachment in the
attached state.

## PCI Capability Boundary

`agent-kernel-x86_64` owns strict PCI capability discovery:

- require the common-header Capability List status bit;
- validate the first capability pointer;
- require aligned offsets inside conventional configuration space;
- bound traversal count;
- reject cycles and duplicate offsets;
- decode MSI, MSI-X, and Virtio vendor capabilities through typed records;
- preserve unrelated configuration bits during mutation.

MSI programming supports one vector per function. The architecture owner:

1. keeps the source disabled;
2. writes the xAPIC message address and data;
3. verifies readback;
4. enables MSI;
5. disables INTx for the proof function.

MSI-X programming uses the capability-described BAR, table offset, and table
size. Each entry is programmed while masked, verified, then unmasked. Function
enable occurs only after the complete table entry is visible.

## xAPIC Message Contract

V28 uses physical xAPIC mode. The typed message encoder accepts:

- an 8-bit destination APIC ID;
- a device vector from `0x20` through `0xdf`;
- fixed delivery mode;
- edge trigger;
- deasserted level.

The encoder produces the PCI MSI address/data pair. Values outside this profile
fail before configuration mutation.

V28 reserves:

| Source | Vector |
| --- | --- |
| QEMU EDU MSI | `0xd0` |
| virtio-rng MSI-X queue 0 | `0xd1` |

Both IDT entries acknowledge the Local APIC. Device-specific cause
acknowledgement remains with the owning driver.

## VT-d Table Expansion

The V27 table owner expands within explicit limits:

- PCI segment zero;
- one PCI bus per table set;
- one VT-d Domain ID per table set;
- up to 256 requester functions on that bus;
- one shared 39-bit second-level hierarchy;
- multiple 4 KiB leaves inside one 2 MiB IOVA window;
- independent requester install and remove;
- independent leaf install and remove.

Requester and IOVA occupancy are derived from live entries plus bounded
metadata. Rebinding a live table set to another bus or Domain fails closed.
Removing one requester leaves the shared leaf mappings and other requester
contexts intact.

## Native Virtio Entropy Driver

The proof uses `virtio-rng-pci-non-transitional`. V28 implements the modern PCI
transport only:

- vendor ID `0x1af4`;
- entropy device ID `0x1044`;
- Virtio PCI Common, Notify, and ISR capabilities;
- `VIRTIO_F_VERSION_1`;
- `VIRTIO_F_ACCESS_PLATFORM`;
- split virtqueue 0;
- one device-writable entropy buffer;
- separate request preparation and Notify transitions;
- MSI-X queue-vector assignment and readback.

The queue uses three mapped pages:

| IOVA | Purpose |
| --- | --- |
| `0x0100_1000` | descriptor, available ring, used ring |
| `0x0100_2000` | entropy output |
| `0x0100_0000` | EDU DMA data |

The descriptor, available ring, and used ring occupy disjoint aligned offsets
inside the queue page. Queue publication uses explicit compiler and CPU memory
ordering before Notify MMIO. The post-detach denial probe writes its entropy
sentinel before Bus Master is reopened and before the prepared queue is
notified.

## QEMU Topology

| Component | Configuration |
| --- | --- |
| machine | Q35, one CPU, 256 MiB |
| IOMMU | Intel VT-d, 39-bit width, interrupt remapping disabled |
| DMA Domain | VT-d Domain ID 1 |
| MSI function | EDU `1234:11e8`, BDF `00:05.0`, requester `0x28` |
| MSI-X function | virtio-rng `1af4:1044`, BDF `00:06.0`, requester `0x30` |
| entropy backend | QEMU random backend |

Interrupt remapping remains disabled so this milestone can isolate PCI
capability programming, Local APIC delivery, and DMA requester translation.

## Activation Order

1. install the two IDT gates with interrupts disabled;
2. map and enable the Local APIC;
3. discover DMAR and both exact PCI identities;
4. probe all required BARs and capabilities;
5. quiesce both PCI functions;
6. create IOMMU, Device, Memory, DMA Domain, and Interrupt Route Resources;
7. attach both requesters to the same Domain;
8. reserve all Core mappings and both interrupt routes;
9. allocate and zero exclusive queue, data, and VT-d table frames;
10. install both requester contexts and every translation leaf;
11. activate VT-d and invalidate caches;
12. activate Core mappings and routes;
13. program MSI and MSI-X while device sources remain disabled;
14. enable PCI memory decode and bus mastering;
15. enable interrupts only around bounded completion waits.

Any failure before step 14 leaves both functions without bus-master authority.
Any failure after step 14 attempts verified quiescence before halt.

## Closed-Loop Proof

The proof must observe all of these outcomes:

1. EDU completes a DMA transfer and raises exactly one MSI on vector `0xd0`.
2. The EDU cause register matches the DMA completion cause.
3. virtio-rng fills the submitted buffer and raises exactly one MSI-X on vector
   `0xd1`.
4. The used-ring identifier and length match the submitted descriptor.
5. Core records two active Device attachments in one DMA Domain.
6. virtio-rng detachment removes requester `0x30` while requester `0x28`
   remains installed.
7. A post-detach virtio DMA probe produces a VT-d fault for requester `0x30`
   and leaves protected memory unchanged.
8. EDU completes another authorized DMA plus MSI after the virtio requester
   has been removed.
9. Both interrupt routes enter released state after hardware masking.

## Failure Semantics

- malformed or cyclic PCI capability chains fail closed;
- unsupported MSI layouts fail before mutation;
- MSI-X tables outside the declared BAR fail before MMIO;
- duplicate live destination/vector pairs fail atomically;
- invalid route transitions leave state and Events unchanged;
- a route Resource or parent Device with live routing rejects retirement;
- detaching and detached requesters do not satisfy new-mapping attachment
  preconditions;
- duplicate active Device or requester attachments fail atomically;
- VT-d requester removal never clears another requester context;
- VT-d leaf removal never clears an unrelated leaf;
- MSI/MSI-X timeouts quiesce both functions;
- an interrupt on the wrong vector fails the proof;
- a completion without the expected device cause fails the proof;
- post-detach target mutation fails the proof;
- a missing or mismatched VT-d fault fails the proof.

## Verification

V28 requires:

- Core interrupt lifecycle, authorization, uniqueness, capacity, retirement,
  attachment, and detach tests;
- PCI capability traversal and malformed-chain tests;
- MSI and MSI-X register-sequence tests;
- xAPIC message encoding tests;
- multi-requester and multi-leaf VT-d table tests;
- modern virtio-rng transport and split-ring tests;
- debug and release V28 QEMU proofs;
- debug and release V27 QEMU regressions;
- debug and release default QEMU regressions;
- workspace tests, strict Clippy, formatting, Supervisor replay, script syntax,
  image audit, and bare-target checks.

## Evidence Markers

```text
AGENT_KERNEL_INTERRUPT_CAPABILITY_OK
AGENT_KERNEL_MULTI_DEVICE_DMA_DOMAIN_OK
AGENT_KERNEL_MSI_CONFIGURED_OK
AGENT_KERNEL_EDU_MSI_DELIVERED_OK
AGENT_KERNEL_MSIX_CONFIGURED_OK
AGENT_KERNEL_VIRTIO_RNG_MSIX_DELIVERED_OK
AGENT_KERNEL_DMA_REQUESTER_DETACHED_OK
AGENT_KERNEL_DMA_DETACH_FAULT_OK
AGENT_KERNEL_SHARED_DOMAIN_SURVIVOR_OK
AGENT_KERNEL_MSI_MSIX_PROOF_OK
```

## References

- [QEMU EDU device specification](https://www.qemu.org/docs/master/specs/edu.html)
- [OASIS Virtio 1.3](https://docs.oasis-open.org/virtio/virtio/v1.3/virtio-v1.3.html)
- [QEMU 11.0.2 EDU implementation](https://gitlab.com/qemu-project/qemu/-/blob/v11.0.2/hw/misc/edu.c)
- [QEMU 11.0.2 Virtio PCI implementation](https://gitlab.com/qemu-project/qemu/-/blob/v11.0.2/hw/virtio/virtio-pci.c)
- [Intel VT-d architecture specification](https://www.intel.com/content/www/us/en/content-details/868911/intel-virtualization-technology-for-directed-i-o-architecture-specification.html)

## Exclusions

V28 leaves interrupt remapping, x2APIC destinations, posted interrupts, queued
invalidation, per-CPU vector migration, MSI multi-message mode, MSI-X
multi-queue steering, ATS, PASID, scalable-mode VT-d, scatter/gather DMA, and
physical device assignment for later milestones.
