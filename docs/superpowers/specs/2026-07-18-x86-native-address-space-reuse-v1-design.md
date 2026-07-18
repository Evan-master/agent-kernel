# X86 Native Address-Space Reuse V1 Design

## Status

Implementation and local validation completed on 2026-07-18. Public
publication is pending.

## Purpose

Address-Space Reclaim V1 transfers every completed native Agent's four private
page-table frames and seven content frames into a bounded 66-frame pool. The
pool can transfer individual frames, while the boot path still creates every
address space from the one-way boot allocator.

Address-Space Reuse V1 closes the next ownership cycle. The reference boot
will atomically withdraw eleven reclaimed frames, rebuild a complete private
x86 address space, execute a newly admitted ring-3 Reuse Worker, clear the
completed address space, and return the same eleven frames. The final pool must
again own 66 unique zeroed frames.

## Atomic Eleven-Frame Allocation

The architecture library adds a read-only allocation preparation operation to
`AddressSpaceFramePool`. Preparation requires at least eleven pooled frames and
captures:

- the exact trailing eleven-frame identity;
- the current pool length;
- the current mutation generation.

The resulting token is copyable planning evidence. Commit revalidates length,
generation, and every selected frame before changing the pool. A successful
commit removes all eleven slots in one mutation and returns a non-copy
`AllocatedAddressSpaceFrames` owner. Stale, reordered, replayed, partial, and
capacity-invalid transfers leave the pool unchanged.

Physical frame zero remains ordinary owned data. Logical occupancy comes from
the pool length, and every selected address must still satisfy alignment,
physical-width, and complete-identity uniqueness rules.

## Rebuilding A Private Address Space

The bare-metal pool accepts allocation only when all selected physical frames
read back as zero. `PreparedAgentMemory::prepare_reused` consumes the allocation
owner and assigns fixed roles in identity order:

| Role | Frames |
| --- | ---: |
| P4 root | 1 |
| P3, P2, P1 | 3 |
| Code | 1 |
| Signal | 1 |
| Stack | 4 |
| Lazy data | 1 |

The constructor initializes content, clones supervisor P4 entries into the
private root, clears the dedicated Agent P4 slot, and maps code, signal, and
stack pages. A fixed page-table allocator supplies exactly P3, P2, and P1 in
that order and rejects any fourth request or incomplete consumption. Lazy data
remains retained and initially unmapped, matching boot-allocated Agent memory.

The rebuilt identity must equal the allocation identity byte for byte. Its
kernel CR3 must match the installed CPU runtime, its Agent root must be
inactive during construction, and every runtime Memory Resource ledger starts
clear.

The 223-event reference profile uses a fixed 2 MiB guarded kernel boot stack.
This capacity covers deterministic bootstrap ownership and the larger bounded
semantic stores without introducing heap-backed kernel state.

## Reuse Worker Lifecycle

After the first six native Agents complete and fill the address-space pool, the
bootstrap Agent admits a dedicated Reuse Worker through public kernel calls:

1. register Agent 10;
2. declare and create one verified Intent and Task;
3. delegate task-scoped authority;
4. register and verify a dedicated Worker Capsule;
5. launch, accept, and queue the Agent;
6. allocate and rebuild one reclaimed address space;
7. execute `DescribeContext`, `SubmitTaskResult`, and `CompleteTask` in ring 3;
8. validate the exact transcript and semantic terminal state;
9. reclaim the completed address space and restore the 66-frame pool.

The Capsule has its own nonce, result, return offsets, digest, and release
artifact hash. A fresh native runtime report isolates this phase from the
preceding six-Agent execution evidence.

## Evidence And Event Contract

The reference boot emits these markers once after their corresponding checks:

```text
AGENT_KERNEL_NATIVE_ADDRESS_SPACE_ALLOCATED_OK
AGENT_KERNEL_NATIVE_ADDRESS_SPACE_REBUILT_OK
AGENT_KERNEL_NATIVE_ADDRESS_SPACE_REUSE_EXECUTION_OK
AGENT_KERNEL_NATIVE_ADDRESS_SPACE_REUSED_RECLAIMED_OK
```

Pool allocation and page-table construction are architecture consequences of
the kernel-visible Agent launch. Final physical teardown is the architecture
consequence of `TaskCompleted`. The semantic layer records Agent, Intent,
Task, Capability, Image, scheduling, result, and completion mutations through
its existing ordered Events.

## Validation

Milestone completion requires:

- red and green host contracts for atomic allocation, stale-token rejection,
  replay rejection, physical frame zero, and exact frame restoration;
- a rebuilt identity that consumes only reclaimed frames;
- exact three-frame private page-table allocation in debug and release builds;
- real ring-3 Reuse Worker execution with an authenticated three-call
  transcript and completed semantic Task;
- final restoration of 66 unique zeroed frames;
- updated deterministic Event totals and strict marker counts;
- workspace tests, Supervisor execution, formatting, `no_std`, bare-metal, and
  warning-free Clippy checks;
- strict debug and release QEMU runs;
- release Capsule and marker artifact audit;
- public `main` publication and remote commit verification.

## Local Validation Evidence

- all five address-space reclamation and allocation host contracts pass;
- `cargo fmt --all --check`, `cargo test --workspace`, and the Supervisor flow
  pass;
- all kernel libraries and the bare-metal binary pass
  `x86_64-unknown-none` checks;
- workspace and bare-metal Clippy pass with warnings denied after permitting
  the existing core-wide `too_many_arguments` baseline;
- strict debug and release QEMU runs each produce exactly 223 ordered Events
  and every required marker exactly once;
- the release ELF contains one 110-byte Reuse Worker Capsule with SHA-256
  `994291663b150574483b987d7733d1b1398802ab73d5865c66fa9b4cf0f06df0`;
- the release Capsule's 78 code bytes match `reuse_worker.S` exactly, with
  return offsets 46, 67, and 76;
- the release Resource Manager and Fault Worker Capsule hashes remain
  `ac5e435801817f5e39debf751ac360999d5e6c0c8e7423e8ceb09c3c1304d6fc`
  and
  `a74bdafa93cb878d578b2dd75ff9b6000d0f6e96ab39d01d658496821aedc4de`.

## Deferred Work

- a general runtime service that creates arbitrary Agent address spaces on
  demand;
- allocation across additional P4, P3, or P2 boundaries;
- multiple concurrent address-space allocation reservations;
- cancellation and rollback after a committed physical allocation;
- SMP synchronization, PCID lifecycle, and hardware TLB shootdown.
