# X86 Native Memory Region V1 Design

## Status

Implemented, validated in debug and release QEMU, and published to public
`main` on 2026-07-18 in commit `40739a0`.

## Purpose

Native Memory Page V0 proved one runtime-controlled physical page in each
prepared Agent. Every address space retained a dedicated frame from boot,
which fixed the maximum allocation at one page and consumed memory even when
the Agent never requested it.

Memory Region V1 introduces a global physical frame pool and bounded
multi-page regions. The kernel chooses every frame, virtual address, flag, and
length. Agents continue to present typed kernel handles and explicit
Capability authority.

The Manager Capsule executes both paths:

```text
allocate, inspect, and release one compatibility page through calls 21-23
    -> create a second owned Memory Resource
    -> allocate a three-page region through call 24
    -> write distinct proofs into the first and last pages from ring 3
    -> inspect both proofs through call 25
    -> release and clear the complete region through call 26
    -> complete only after the global pool and private mappings are clear
```

This milestone converts runtime physical memory into a shared kernel service
and establishes a region ABI suitable for later allocators, package runtimes,
and storage or network buffers.

## Capacity Contract

The first implementation uses explicit fixed capacities:

- 16 physical frames in one global runtime pool;
- 8 virtual region slots per Agent;
- up to 4 active regions per Agent;
- 1 through 4 pages in each region;
- 4096 bytes per page.

The compatibility page keeps virtual base `0x0000400000008000`. The region
arena starts at `0x0000400000009000` and spans eight leaves. Region allocation
uses deterministic first-fit placement within the calling Agent address
space. Physical frames use deterministic first-fit selection from the global
pool.

All region leaves fit under page-table intermediates created during Agent
address-space preparation. Dynamic intermediate allocation remains a later
memory-manager stage.

## Physical Ownership

`RuntimeFramePool` is prepared once after all private Agent memories. It
removes 16 distinct frames from BootInfo `Usable` regions, clears every byte,
and proves disjointness from all Agent roots and content frames.

Each pool slot follows this lifecycle:

```text
Available
    -> Reserved(agent, resource, transaction)
    -> Mapped(agent, resource, memory_cell, generation, transaction)
    -> Available
```

A reservation contains one through four frame indices under one transaction
token. Commit and cancellation validate the complete frame set, so partial
ownership transitions cannot enter the reusable pool. Release clears every
frame before its state returns to `Available`.

The compatibility page also obtains its frame from this pool. Prepared Agent
memory therefore retains code, signal, stack, and demand-page frames only.

## Virtual Region Ownership

Each `PreparedAgentMemory` owns one `RuntimeRegionLedger`. The ledger tracks
four region records and an eight-bit virtual-slot map. Its lifecycle is:

```text
Available entry and free contiguous slots
    -> Reserved(resource, base slot, page count, generation, token)
    -> Mapped(resource, memory_cell, base slot, page count, generation, token)
    -> Available entry and free slots
```

Only one reservation may be in preparation at a time for an Agent. Multiple
committed regions can coexist. Cancellation releases virtual slots without
advancing the committed generation. Successful release advances the ledger
and rejects stale reservation or release tokens.

The canonical MemoryCell descriptor remains four words:

```text
word 0 = kernel-selected virtual base
word 1 = byte length
word 2 = access code 3 (user read + write, execute disabled)
word 3 = allocation generation
```

## Authority Contract

Allocation requires:

- the authenticated scheduler-selected running Agent context;
- an active owned `ResourceKind::Memory` Resource;
- an active caller Capability on that exact Resource with `Act`;
- an accepted page count from 1 through 4;
- free contiguous virtual slots, free pool frames, one MemoryCell slot, and
  one Event slot.

Inspection requires:

- `Observe` on the descriptor's exact Memory Resource;
- a live region and pool binding for the Agent, Resource, MemoryCell, page
  count, and generation;
- canonical descriptor words and exact user-writable NX leaf mappings.

Release requires:

- `Rollback` on the descriptor's exact Memory Resource;
- the same complete region and pool binding;
- prevalidated page-table leaves and physical frames;
- one Event slot.

Zero handles, stale tokens, mismatched ownership, malformed page counts,
altered descriptors, retired Resources, unknown operations, and nonzero
reserved registers fail closed.

## Agent Call ABI

ABI version 1 gains three register-only operations:

| Operation | ID | Request payload | Success reply |
| --- | ---: | --- | --- |
| `AllocateMemoryRegion` | 24 | `r10=Capability`, `r11=Memory Resource`, `r12=page count` | `r10=MemoryCell`, `r11=base`, `r12=bytes`, `r13=page count`, `r14=generation` |
| `InspectMemoryRegion` | 25 | `r10=Capability`, `r11=MemoryCell` | `r10=MemoryCell`, `r11=first proof`, `r12=last proof`, `r13=page count`, `r14=generation` |
| `ReleaseMemoryRegion` | 26 | `r10=Capability`, `r11=MemoryCell` | `r10=MemoryCell`, `r11=Resource`, `r12=page count`, `r13=generation` |

Calls 21 through 23 retain their wire contract and move to pooled frames.
Every unused extension register remains zero. Replies preserve Agent, Task,
Image, and nonce identity.

## Transaction Boundaries

Allocation follows this sequence:

1. validate context, authority, page count, semantic capacities, virtual
   capacity, and physical capacity;
2. reserve pool frames and virtual slots;
3. install and validate every private leaf;
4. call `sys_create_memory_cell` through the public facade;
5. commit the virtual and physical bindings;
6. encode the authenticated reply.

Steps 2 and 3 are reversible. A facade rejection removes every temporary
leaf, verifies frame contents, and cancels both reservations. A mismatch after
the facade commit terminates the validation boot.

Inspection validates all leaves and ownership records, reads the first 64-bit
word of the first and last frames through supervisor aliases, calls
`sys_recall_memory_cell`, and encodes one audited reply.

Release prepares immutable virtual and physical cleanup tokens before calling
`sys_retire_resource`. After the semantic commit, the kernel removes all
leaves, clears all frame bytes, commits both ledgers, and encodes the reply.
Task completion requires zero active pool leases for the Agent. Fault capture
also requires clear runtime-memory ledgers in this stage; automatic retirement
of semantically active Resources remains a dedicated recovery milestone.

## Page-Table Contract

Every compatibility-page and region mapping uses 4 KiB leaves with:

- `PRESENT`;
- `USER_ACCESSIBLE`;
- `WRITABLE`;
- `NO_EXECUTE`.

Bulk activation prevalidates the complete target range and rolls back all
installed leaves on any validation failure. Bulk deactivation captures all
entries, removes the complete range, validates absence, and restores the
captured entries if absence validation fails.

The current CR3 contract continues to accept PWT and PCD control bits. PCID
support requires a later generation-aware invalidation protocol.

## Layer Placement

- `agent-kernel-core` retains Resource, Capability, MemoryCell, and Event
  semantics with no physical-frame concepts.
- `agent-kernel` continues to expose all semantic mutations through public
  facade methods.
- the x86 architecture library owns pure pool and region ledgers, capacities,
  virtual layout, and ABI types.
- the bare-metal Agent-memory layer owns BootInfo frame extraction, supervisor
  aliases, page-table transitions, zeroing, and physical evidence.
- the native executor coordinates reversible physical preparation around
  facade commits.
- the Manager Capsule proves the complete lifecycle from ring 3.

## QEMU Evidence

The strict debug and release boot proof requires:

- a 16-frame pool disjoint from all prepared Agent memory;
- pooled execution of compatibility calls 21 through 23;
- a three-page region created through calls 24 through 26;
- distinct ring-3 writes observed in the first and last frames;
- exact canonical descriptors for both MemoryCells;
- two retired Memory Resources with exact Capabilities;
- absent compatibility and region leaves after release;
- all 16 frames available and fully zeroed at Manager completion;
- the exact 22-operation Manager transcript and return offsets;
- ten ordered memory lifecycle Events across the two resources;
- dedicated pool, region allocation, inspection, release, and terminal
  markers.

## Validation

The completed implementation passed formatting, all workspace tests, the host
Supervisor flow, all kernel-crate `x86_64-unknown-none` checks, and the
freestanding bare-metal build. Host and bare x86 scoped Clippy passed with
warnings denied while allowing the inherited `too_many_arguments` lint class.
Strict lint triage found eight unchanged core findings and one unchanged x86
boundary constructor in that class; no crate-wide suppression was added.

Debug and release QEMU both completed the exact 190-event transcript. Each run
proved compatibility-page calls 21 through 23, region calls 24 through 26,
three-page first/last ring-3 writes, 44 Manager address-space switches, zero
active runtime leases, all 16 pool frames available and zeroed, and the final
Driver invocation.

Fresh assembly extraction produced a 1766-byte Manager program. Prefixing the
32-byte Capsule header reproduced all 1798 checked-in bytes and SHA-256
`f45fef6faceb2b1cb16c7922048db1df5d1b23ab06bb60f2132fc73271a8de51`.
Disassembly reproduced all 22 return offsets and retained full register-frame
capture, Agent/kernel CR3 transitions, and `iretq` restoration in the release
ELF.

## Deferred Work

- dynamic page-table intermediate allocation;
- shared-memory grants and explicit cross-Agent mappings;
- executable mapping policy for verified code loaders;
- fault-time Resource retirement and frame reclamation;
- complete Agent address-space destruction;
- copy-on-write, swapping, overcommit, DMA, IOMMU, and NUMA policy;
- hardware TLB shootdown for SMP.
