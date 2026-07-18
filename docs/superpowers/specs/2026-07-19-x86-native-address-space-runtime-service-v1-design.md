# X86 Native Address-Space Runtime Service V1 Design

## Status

Implemented and validated on 2026-07-19. Publication is pending.

## Purpose

Address-Space Reuse V1 proves one complete physical ownership cycle after the
initial six native Agents terminate. Its boot adapter still performs allocation,
memory construction, CPU preparation, runtime registration, execution, and
reclamation as one hard-coded sequence.

Runtime Service V1 turns that sequence into a reusable single-core kernel
service. The reference boot must admit two additional ring-3 Agents while both
private address spaces are live, preserve Agent-bound ownership through every
type-state transition, recover all frames from a rejected admission, execute
both tasks through kernel-selected FIFO dispatch, and restore the final
66-frame pool.

## Agent-Bound Frame Ownership

`AddressSpaceFramePool` allocation preparation accepts one nonzero `AgentId`.
The copyable preparation token binds:

- the target Agent;
- the exact trailing eleven-frame identity;
- the current pool length;
- the current pool generation.

Commit revalidates the complete token and returns a non-copy
`AllocatedAddressSpaceFrames` owner containing the same Agent and identity.
Two committed owners may coexist only when their complete frame identities are
disjoint. Invalid Agents, stale tokens, replays, partial capacity, or changed
frame order leave the pool unchanged.

Cancellation consumes one allocation owner and attempts to return all eleven
frames in one mutation. A failed cancellation returns the same owner to the
caller, preserving physical ownership. The bare-metal adapter clears every
frame and verifies all bytes before committing a cancellation to the pool.

## Transactional Runtime Admission

`NativeAddressSpaceService` coordinates four existing owners:

1. the reclaimed frame pool;
2. `PreparedAgentMemory`;
3. `AgentCpuRuntime`;
4. `NativeAgentRuntime`.

Admission derives its target Agent from the authenticated `AgentCallContext`.
It allocates one Agent-bound frame owner, rebuilds content and private page
tables, prepares the CPU context, and registers the non-running CPU. The
resulting admission evidence records Agent and complete physical identity.

`PreparedAgentMemory` retains the allocation Agent. CPU preparation requires
that identity to match the call context. Fallible memory construction returns
the original allocation owner. Fallible CPU preparation returns the prepared
memory. Rejected runtime registration returns the prepared CPU. Each failure
path clears and atomically restores the complete address space before returning
bounded failure evidence.

These physical mutations are architecture consequences of semantic launch and
admission state already recorded through public kernel calls. Cancellation
does not add a semantic Event because the queued Task and Agent lifecycle stay
unchanged; its physical proof is carried by service evidence and strict QEMU
markers.

## Concurrent Batch Lifecycle

After the initial six address spaces fill the pool, the bootstrap Agent prepares
two independent Reuse Worker flows:

| Property | Worker A | Worker B |
| --- | ---: | ---: |
| Agent | 10 | 11 |
| Intent | distinct | distinct |
| Task | distinct | distinct |
| Delegated Capability | distinct | distinct |
| Registered Image record | distinct | distinct |
| Capsule bytes | shared immutable contract | shared immutable contract |

The service admits Worker A, then attempts a duplicate Worker A admission. The
runtime registry rejects the duplicate physical context and the service returns
its eleven frames. Worker B is then admitted. Before execution:

- both Workers own disjoint complete identities;
- the runtime registry owns exactly two prepared CPUs;
- the frame pool owns 44 unique zeroed frames;
- the semantic run queue contains Worker A followed by Worker B.

With a one-tick quantum, FIFO execution must produce four dispatches and two
physical quantum expiries. Each Worker performs `DescribeContext`,
`SubmitTaskResult`, and `CompleteTask`, producing three calls and six
Agent/kernel address-space switches. Both Tasks are then verified, both Intents
become fulfilled, and batch reclamation returns 22 frames.

## Deterministic Reference Profile

The completed profile must prove:

| Evidence | Count |
| --- | ---: |
| Registered Agents | 11 |
| Native ring-3 completions | 8 |
| Kernel-selected dispatches | 27 |
| Runtime-service Worker calls | 6 |
| Runtime-service Worker address-space switches | 12 |
| Physical quantum expiries | 12 |
| Address-space cancellation cycles | 1 |
| Frames restored by admission cancellation | 11 |
| Terminal address-space reclamations | 8 |
| Terminal private-frame returns | 88 |
| Final zeroed private frame pool | 66 |
| Capabilities | 21 |
| Intents | 9 |
| Tasks | 9 |
| Ordered kernel Events | 241 |

## QEMU Evidence

The reference boot retains the four Reuse V1 markers and adds these service
proofs:

```text
AGENT_KERNEL_NATIVE_ADDRESS_SPACE_RUNTIME_CANCEL_OK
AGENT_KERNEL_NATIVE_ADDRESS_SPACE_RUNTIME_BATCH_OK
AGENT_KERNEL_NATIVE_ADDRESS_SPACE_RUNTIME_CONCURRENCY_OK
```

Every marker must appear exactly once in debug and release builds. The script
must validate the complete 241-event sequence and reject extra events, missing
events, duplicate markers, fail-closed paths, and unexpected QEMU exit status.

## Validation

Milestone completion requires:

- red and green host contracts for Agent binding, concurrent owners, stale
  allocation rejection, atomic cancellation, and failed-cancellation ownership;
- a duplicate runtime admission that restores one complete modified address
  space without disturbing the first live Agent;
- two simultaneously registered, disjoint prepared Agent CPUs;
- exact FIFO execution, call transcripts, results, completion, verification,
  Intent fulfillment, and batch reclamation;
- final restoration of 66 unique zeroed frames;
- formatting, workspace tests, Supervisor execution, all kernel-library
  `no_std` checks, the bare-metal check, and scoped Clippy with warnings denied;
- strict debug and release QEMU runs;
- release Capsule, marker, and source-byte artifact audit;
- public `main` publication and remote commit verification.

## Local Validation Evidence

- all seven address-space reclamation and allocation host contracts pass,
  including Agent binding, concurrent disjoint owners, atomic cancellation,
  and failed-cancellation ownership retention;
- `cargo fmt --all --check`, the complete workspace test suite, and the
  Supervisor reference flow pass;
- all kernel libraries and the bare-metal binary pass
  `x86_64-unknown-none` checks;
- workspace and bare-metal Clippy pass with warnings denied after permitting
  the existing core-wide `too_many_arguments` baseline;
- strict debug and release QEMU runs each produce exactly 241 ordered Events,
  and every required address-space marker appears exactly once;
- the release ELF contains one 110-byte Reuse Worker Capsule with SHA-256
  `994291663b150574483b987d7733d1b1398802ab73d5865c66fa9b4cf0f06df0`;
- the Capsule's 78 code bytes match a fresh assembly of `reuse_worker.S`, with
  return symbols at offsets 46, 67, and 76;
- the release Resource Manager and Fault Worker Capsules each occur once and
  retain SHA-256
  `ac5e435801817f5e39debf751ac360999d5e6c0c8e7423e8ceb09c3c1304d6fc`
  and
  `a74bdafa93cb878d578b2dd75ff9b6000d0f6e96ab39d01d658496821aedc4de`.

## Deferred Work

- an asynchronous semantic syscall that requests address-space admission from
  a long-lived userspace Supervisor;
- dynamic page-table hierarchy growth across additional P4, P3, or P2 ranges;
- admission queues larger than the fixed six-context native runtime;
- task-cancellation reclamation for actively running or faulted CPUs;
- SMP synchronization, PCID lifecycle, and hardware TLB shootdown.
