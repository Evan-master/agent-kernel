# X86 Native Memory Concurrency V1 Design

## Status

Implemented, validated in debug and release QEMU, and published to public
`main` on 2026-07-18 in commit `ae9b738`.

## Purpose

Native Memory Region V1 established a shared 16-frame pool, eight virtual
region slots per Agent, four active-region records, and Agent Calls 24 through
26. Its QEMU path exercised one three-page region at a time. The pure ledger
tests already covered concurrent records and first-fit hole reuse.

Memory Concurrency V1 carries those contracts into real ring-3 execution. The
Manager Capsule keeps two regions active together, releases the lower region,
and allocates a third region into the resulting virtual hole while the upper
region remains mapped.

## Manager Sequence

The existing compatibility-page lifecycle remains unchanged. The region phase
uses three owned Memory Resources:

```text
create Region A Resource
    -> allocate A as three pages at slots 0..2, generation 1
    -> write and inspect A first/last proofs
    -> create Region B Resource
    -> allocate B as two pages at slots 3..4, generation 2
    -> write B first/last proofs while A remains live
    -> release A
    -> inspect B to prove its mapping survived A release
    -> create Region C Resource
    -> allocate C as three pages at slots 0..2, generation 3
    -> write and inspect C first/last proofs while B remains live
    -> release B
    -> release C
    -> submit the Manager result and complete
```

The kernel selects every virtual address and physical frame. The Capsule checks
the exact returned bases, byte lengths, page counts, generations, handles, and
proof values before issuing the next call.

## First-Fit Contract

The fixed region arena begins at `0x0000400000009000`.

| Region | Pages | Expected base | Generation |
| --- | ---: | ---: | ---: |
| A | 3 | `0x0000400000009000` | 1 |
| B | 2 | `0x000040000000c000` | 2 |
| C | 3 | `0x0000400000009000` | 3 |

At B allocation, A occupies slots 0 through 2, so deterministic first fit
selects slots 3 and 4. Releasing A frees the lower three slots. C then reuses
those exact slots while B continues to own slots 3 and 4.

The physical pool may reuse released frame indices under its existing global
first-fit policy. Every reused frame must be fully zero before reservation.

## Observation Log

One terminal `Option` cannot prove three independent inspections. The x86
architecture library gains a fixed-capacity `RuntimeRegionObservationLog` with
three ordered entries. Each entry contains:

- MemoryCell;
- virtual start slot;
- page count;
- allocation generation;
- first-page proof;
- last-page proof.

Recording requires a live `RuntimeRegionBinding`. Zero handles, invalid page
counts, duplicate MemoryCells, non-monotonic generations, and a fourth entry
are rejected without mutation. The log remains copyable, allocation-free, and
owned by `PreparedAgentMemory`; the completed CPU transfers the exact array to
terminal evidence.

The expected proof values are:

| Region | First page | Last page |
| --- | --- | --- |
| A | `0x524547494f4e3031` (`REGION01`) | `0x524547494f4e3033` (`REGION03`) |
| B | `0x524547494f4e4231` (`REGIONB1`) | `0x524547494f4e4232` (`REGIONB2`) |
| C | `0x524547494f4e4331` (`REGIONC1`) | `0x524547494f4e4333` (`REGIONC3`) |

## Authority And Events

Each region uses a distinct owned `ResourceKind::Memory` Resource and a root
Capability containing `Observe`, `Act`, and `Rollback`. Allocation, inspection,
and release continue to call the public MemoryCell and Resource facades.

Each added region contributes the same five ordered events:

1. `ResourceCreated`;
2. `CapabilityGranted`;
3. `MemoryCellCreated`;
4. `MemoryCellRecalled`;
5. `ResourceRetired`.

Region B and Region C add ten events to the reference boot. Final fixed
capacities become six Resources, seventeen Capabilities, four MemoryCells, and
two hundred Events.

## Transcript Contract

The Manager transcript grows from 22 to 30 operations. It contains one initial
`DescribeContext`, the existing management and compatibility-page calls, three
complete region lifecycles with the interleaved release order above, one
`SubmitTaskResult`, and one `CompleteTask`.

Thirty physical Agent Calls produce sixty Agent/kernel address-space switches.
The regenerated Capsule bytes, digest, operation list, and all return offsets
remain checked into the auditable Manager image module.

## Release Safety

Releasing A must consume only A's virtual and physical tokens. B's descriptor,
leaf mappings, pool binding, and proof values remain valid. Releasing B while C
is active follows the same rule. A stale token is rejected before any frame
bytes are cleared.

Task completion still requires:

- no active compatibility-page mapping;
- no active region records;
- all eight region leaves absent;
- no pool lease owned by the Manager;
- all sixteen pool frames available and zero.

## QEMU Evidence

Strict debug and release boot validation requires:

- exact A, B, and C bases, lengths, page counts, generations, and handles;
- A and B concurrently mapped before A release;
- B still inspectable after A release;
- C reusing A's three virtual slots while B remains mapped;
- three ordered observation-log entries with six exact proof values;
- exact release order A, B, C;
- 30 Manager operations and 60 address-space switches;
- six Resources, seventeen Capabilities, four MemoryCells, and 200 Events;
- all runtime leaves absent and all pool frames available and zero at Manager
  completion;
- unchanged fault, verifier, Driver, and final handoff evidence.

## Validation

The completed implementation passed formatting, all workspace tests, the host
Supervisor flow, all kernel-library `x86_64-unknown-none` checks, and the full
freestanding bare-metal build. Workspace and bare x86 Clippy passed with
warnings denied while allowing the inherited `too_many_arguments` lint class.
The forbidden host-API and allocation scan returned no matches in `no_std`
source trees.

Debug and release QEMU both completed the exact 200-event transcript. Each run
performed three region allocations, three inspections, and three releases. The
terminal proof bound A to slots 0 through 2 at generation 1, B to slots 3 and 4
at generation 2, and C to the reused slots 0 through 2 at generation 3. All 16
physical frames were available and zero at completion.

Fresh assembly produced a 2656-byte Manager program. The 32-byte Capsule header
produced a 2688-byte image with SHA-256
`14f39bdc29b4e4b0fcc820f6b7b0b1e4ff733c3cd45715f8e4413b18ddbe1499`.
The release ELF contained the exact same bytes. Disassembly reproduced all 30
return offsets:

```text
45, 86, 163, 236, 310, 390, 463, 539, 626, 710,
794, 878, 968, 1042, 1161, 1254, 1344, 1421, 1567, 1695,
1772, 1918, 2012, 2140, 2217, 2363, 2485, 2571, 2645, 2654
```

## Deferred Work

- fault-time retirement and reclamation of active memory Resources;
- dynamic page-table intermediate allocation;
- shared-memory grants and cross-Agent mappings;
- complete Agent address-space destruction;
- executable verified-image mappings;
- SMP TLB shootdown, copy-on-write, swapping, overcommit, DMA, IOMMU, and NUMA
  policy.
