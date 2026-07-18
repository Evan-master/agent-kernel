# X86 Native Memory Page V0 Design

## Status

Implemented and validated in debug and release QEMU on 2026-07-18.
Publication is pending.

## Purpose

The current x86 runtime gives every admitted Agent a private code page, signal
page, stack, and one retained demand page. Those pages are fixed during boot.
Native Memory Page V0 adds the first runtime-controlled physical page
lifecycle to the Agent Call boundary.

The Manager Capsule will execute this sequence:

```text
create an owned Memory Resource with Observe + Act + Rollback authority
    -> allocate one kernel-selected private 4 KiB page
    -> receive its MemoryCell handle and virtual address
    -> write a proof value from ring 3
    -> inspect the value through an audited Agent Call
    -> retire the Memory Resource
    -> clear and remove the page mapping before task completion
```

This milestone establishes a real physical and virtual memory control path.
The bounded single-page slot keeps allocation, ownership, page-table updates,
and terminal evidence inspectable while later milestones add pools and
multi-page regions.

## Native Object Binding

The lifecycle composes two existing core objects:

- `ResourceKind::Memory` owns authority and terminal state;
- `MemoryCell` stores the architecture-neutral page descriptor.

The descriptor words are canonical:

```text
word 0 = kernel-selected virtual base
word 1 = byte length (4096)
word 2 = access code (user read + write, execute disabled)
word 3 = allocation generation
```

Creation uses `sys_create_resource`. Allocation records the descriptor through
`sys_create_memory_cell`. Inspection uses `sys_recall_memory_cell`. Release
uses `sys_retire_resource`. Existing core authorization and event semantics
therefore remain authoritative.

Ring-3 page writes are ordinary private CPU memory accesses. Kernel lifecycle
operations remain represented by `ResourceCreated`, `CapabilityGranted`,
`MemoryCellCreated`, `MemoryCellRecalled`, and `ResourceRetired` events.

## Authority Contract

The three new calls require the scheduler-authenticated running Agent context.

Allocation requires:

- an active `ResourceKind::Memory` Resource;
- an active Capability owned by the caller;
- `Act` authority on that exact Resource;
- one available runtime page slot;
- one available MemoryCell slot and one event slot.

Inspection requires:

- `Observe` authority on the descriptor's Memory Resource;
- an active mapping bound to the exact Resource and MemoryCell;
- an unchanged canonical descriptor;
- a live user-writable, non-executable leaf mapping.

Release requires:

- `Rollback` authority on the descriptor's Memory Resource;
- an active mapping bound to the exact Resource and MemoryCell;
- a prevalidated page-table leaf and exclusive physical frame;
- one available event slot.

Zero identifiers, stale nonces, mismatched handles, retired Resources,
unknown operation values, and non-zero reserved registers fail closed.

## Runtime Page Ownership

Each prepared Agent address space gains one exclusive retained runtime frame.
The frame is removed from a BootInfo `Usable` region during preparation and is
included in pairwise physical-disjointness evidence. Its user leaf starts
absent.

`RuntimePageLedger` owns a deterministic lifecycle:

```text
Available(generation)
    -> Reserved(resource, next_generation)
    -> Mapped(resource, memory_cell, generation)
    -> Available(generation)
```

A failed semantic commit removes the temporary leaf, clears the frame, and
cancels the reservation. Successful release validates the leaf and frame
before the core retirement commit, then removes the leaf and clears all 4096
bytes. The next allocation may reuse the same physical frame with a higher
generation.

The runtime virtual address is a fixed page directly after the retained lazy
data page. Agents never supply raw virtual or physical addresses.

The address-space contract permits only CR3 PWT and PCD control bits. PCID
values are rejected, so every switch to the private root provides the required
non-global translation invalidation until a later PCID-aware TLB protocol is
implemented.

## Agent Call ABI

The ABI remains version 1 and register-only.

| Operation | ID | Request payload | Success reply |
| --- | ---: | --- | --- |
| `AllocateMemoryPage` | 21 | `r10=Capability`, `r11=Memory Resource` | `r10=MemoryCell`, `r11=virtual base`, `r12=4096`, `r13=generation` |
| `InspectMemoryPage` | 22 | `r10=Capability`, `r11=MemoryCell` | `r10=MemoryCell`, `r11=first u64 value`, `r12=generation` |
| `ReleaseMemoryPage` | 23 | `r10=Capability`, `r11=MemoryCell` | `r10=MemoryCell`, `r11=Resource`, `r12=generation` |

All remaining extension registers are reserved and must be zero. Every reply
retains the authenticated Agent, Task, Image, and nonce fields.

## Transaction Boundaries

Allocation follows a reversible sequence:

1. validate authenticated context, Resource, Capability, capacities, and the
   architecture slot;
2. reserve the ledger and install the private leaf;
3. call `sys_create_memory_cell` through the public facade;
4. bind the returned handle and encode the reply;
5. remove and clear the mapping if step 3 or 4 fails.

Inspection validates physical and semantic identity before appending the
audited recall event. It then returns the first 64-bit value read through the
kernel's supervisor alias of the same exclusive frame.

Release builds a validated cleanup token before calling
`sys_retire_resource`. Consuming that token removes the exact leaf, clears the
frame, advances the ledger, and makes the slot reusable. Any unexpected
post-commit hardware mismatch terminates the validation boot.

## Layer Placement

- `agent-kernel-core` retains Resource, Capability, MemoryCell, and Event
  semantics without x86 addresses or physical frames.
- `agent-kernel` exposes the existing MemoryCell facade and a BootedKernel
  capacity selected by the architecture profile.
- `agent-kernel-x86_64` owns the pure runtime-page ledger, virtual layout,
  physical frame, page-table leaf, ABI decoding, and hardware evidence.
- The bare-metal executor coordinates reversible architecture preparation with
  public facade commits.
- The Manager Capsule performs the complete lifecycle from ring 3.

## QEMU Evidence

The strict boot proof will require:

- one owned Memory Resource in terminal state;
- one canonical MemoryCell descriptor;
- one physical frame exclusive from every other Agent frame;
- a ring-3 proof write observed through the supervisor alias;
- an absent user leaf and a fully zeroed retained frame after release;
- operations 21 through 23 in the exact Manager transcript;
- the five ordered core events associated with resource creation, allocation,
  inspection, and release;
- dedicated allocation, inspection, release, and terminal memory-manager
  markers in debug and release QEMU.

## Deferred Work

- a shared runtime frame pool spanning multiple Agent address spaces;
- multi-page regions and page-count negotiation;
- dynamic page-table intermediate allocation;
- shared-memory grants between Agents;
- copy-on-write, swapping, overcommit, DMA, and NUMA policy;
- architecture-independent transactional memory-region records;
- reclaim of complete Agent address spaces after retirement.
