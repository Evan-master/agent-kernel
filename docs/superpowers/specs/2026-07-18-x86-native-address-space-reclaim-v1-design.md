# X86 Native Address-Space Reclaim V1 Design

## Status

Implementation and local validation completed on 2026-07-18. Publication is
tracked in the milestone plan.

## Purpose

Native task completion already retires every live runtime Memory Resource and
returns its pooled frames. The completed CPU currently extracts transcript
metadata and drops `PreparedAgentMemory`. Its code, signal, stack, lazy-data,
root, and private intermediate page-table frames remain reserved in physical
memory with no reusable ownership record.

Address-Space Reclaim V1 closes that terminal ownership gap. Every completed
native CPU retains its address-space owner until all semantic and transcript
evidence has been checked. The boot executor then clears all private frames,
verifies every byte, and commits the frame set to a bounded reusable pool.

## Complete Physical Identity

Each native address space owns eleven frames:

| Frame class | Count |
| --- | ---: |
| P4 root | 1 |
| Private P3, P2, and P1 tables | 3 |
| Code | 1 |
| Signal | 1 |
| Stack | 4 |
| Lazy data | 1 |
| Total | 11 |

`AgentMemoryIdentity` records all four private table frames and all seven
content frames. Construction rejects unaligned and out-of-range addresses,
duplicate table frames, and table/content aliases. Physical frame zero remains
valid owned data because the x86 boot memory map may expose it as usable.
Cross-Agent disjointness compares all eleven owned frames. Runtime-pool
disjointness uses the same complete identity.

The bare-metal page-table installer wraps the boot allocator while mapping the
fixed user layout. It must observe exactly three intermediate allocations in
addition to the explicit root allocation. The resulting frame order is stable:
P4, P3, P2, P1.

## Bounded Reclamation Pool

The architecture library exposes a const-generic `AddressSpaceFramePool`. A
read-only prepare operation validates:

- capacity for one complete eleven-frame set;
- complete identity validity;
- absence of every frame from the current pool;
- the exact expected pool length and mutation generation.

Prepare returns a copyable token. Commit accepts the token only at its expected
pool length and generation, which rejects stale, reordered, and replayed
commits even after the pool has been drained. Frames may later be taken from
the pool for a future native address-space allocator. The reference boot uses
capacity 66 for six native Agents.

## Terminal Reclamation

`CompletedAgentCpu` retains `PreparedAgentMemory` after `TaskCompleted`. The
runtime report keeps all six completed owners while Worker, Verifier, Fault
Handler, Fault Worker, and Resource Manager evidence is inspected.

After the final Manager proof, reclamation follows one fail-closed sequence:

1. require an empty native runtime registry and six completed CPUs;
2. require clear compatibility-page and region ledgers for every owner;
3. require the kernel CR3 and a non-active Agent root;
4. preflight all six pool commits against a scratch pool copy;
5. clear seven content frames and four private page-table frames per Agent;
6. read every byte back as zero;
7. commit each eleven-frame token to the real pool;
8. require an empty completion report and 66 unique zeroed pooled frames.

The private root contains cloned supervisor entries. Clearing the private root
removes those references without writing any shared lower kernel table. The
three table frames below the dedicated Agent P4 slot are exclusively owned and
are cleared directly through the supervisor physical window.

## Evidence And Event Contract

The reference boot emits these physical proof markers once:

```text
AGENT_KERNEL_NATIVE_ADDRESS_SPACE_RECLAIMED_OK
AGENT_KERNEL_NATIVE_ADDRESS_SPACE_FRAME_POOL_OK
```

The semantic totals remain seven Resources, nineteen Capabilities, five
MemoryCells, and 205 Events. Address-space teardown is the architecture-level
physical consequence of the existing `TaskCompleted` events, so it adds no
second semantic lifecycle event.

## Validation

Milestone completion requires:

- red and green complete-identity and reclamation-pool host contracts;
- exact private page-table allocation tracking in debug and release builds;
- workspace tests and Supervisor execution;
- `no_std`, bare-metal, formatting, and warning-free Clippy checks;
- strict debug and release QEMU runs with 205 ordered Events;
- six reclaimed address spaces, 66 unique frames, and full zero verification;
- unchanged Resource Manager and Fault Worker Capsule digests;
- public `main` publication and remote commit verification.

## Release Artifact Evidence

Strict debug and release QEMU runs each completed with 205 ordered Events and
one occurrence of each address-space proof marker. Six completed address
spaces transferred 24 private page-table frames and 42 content frames into the
bounded pool, for 66 unique frames total. The physical readback accepted only
fully zeroed frames.

The optimized release ELF retained the established Agent Image Capsule bytes:

| Capsule | Bytes | SHA-256 |
| --- | ---: | --- |
| Resource Manager | 2,594 | `ac5e435801817f5e39debf751ac360999d5e6c0c8e7423e8ceb09c3c1304d6fc` |
| Fault Worker | 325 | `a74bdafa93cb878d578b2dd75ff9b6000d0f6e96ab39d01d658496821aedc4de` |

Both digests match the preceding native completion-memory reclaim milestone.

## Deferred Work

- allocation of a new native address space from reclaimed frames;
- runtime growth across additional P4, P3, or P2 boundaries;
- cancellation-time address-space reclamation;
- suspended and retired Agent teardown;
- SMP synchronization and hardware TLB shootdown.
