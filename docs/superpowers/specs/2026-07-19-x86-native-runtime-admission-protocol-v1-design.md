# X86 Native Runtime Admission Protocol V1 Design

## Status

Implemented and validated on 2026-07-19.

## Purpose

Native Address-Space Runtime Service V1 owns transactional physical admission,
yet its reference boot invokes the service directly. The semantic kernel has no
object that records which Agent requested a target runtime, which authority was
used, or how the architecture completed the request.

Runtime Admission Protocol V1 adds that missing Agent-native control plane. A
real ring-3 Supervisor submits requests through Agent Call 27. The deterministic
kernel validates authority and target lifecycle, records ordered requests, and
issues generation-bound permits. The x86 broker consumes those permits through
the existing physical service, then commits semantic admission and FIFO queueing
as one bounded kernel mutation.

## Semantic Object

`RuntimeAdmissionRecord` is a fixed-width, replayable kernel object with:

- `RuntimeAdmissionId`;
- requesting Supervisor Agent;
- authorizing `Delegate` Capability;
- target Agent, Task, Image, and Resource;
- `Requested`, `Admitted`, or `Rejected` status;
- an optional bounded physical failure classification.

The store uses the existing `TASKS` capacity. One launched task can own at most
one admission record, and records remain available after terminal status for
audit. No heap allocation, host state, timestamps, or hidden global ownership
is introduced.

## Request Authority

The request syscall accepts requester, root-scoped authority, target Agent, and
target Task. It requires all of the following before mutation:

1. the requester is active and has a launched `Supervisor` entry;
2. the authority belongs to the requester, targets the Task Resource, has an
   active ancestry chain, carries `Delegate`, and has no Task scope;
3. the target Agent is active and has a launch entry matching the accepted Task;
4. the target entry resolves one verified image and valid task-scoped `Act`
   authority;
5. the Task is accepted, assigned to the target, and absent from the run queue;
6. store, identifier, generation, and event capacities are available;
7. no prior admission record owns the same target or Task.

Successful creation emits `RuntimeAdmissionRequested`. Every rejected request
leaves records, identifiers, generation, queue state, and Events unchanged.

## Kernel Permit And Commit

The architecture requests the oldest pending record through a read-only
preparation method. `RuntimeAdmissionPermit` binds the complete record and the
current admission generation. Commit revalidates every field and preflights two
Event slots plus one run-queue slot.

After physical admission succeeds, one core commit:

- marks the record `Admitted`;
- advances the admission generation;
- emits `RuntimeAdmissionAdmitted`;
- queues the accepted target Task;
- emits `TaskQueued`.

The pair is capacity-preflighted before either mutation. A stale permit, changed
target lifecycle, revoked authority, occupied queue slot, or insufficient Event
capacity leaves the complete semantic state unchanged.

Physical admission failure consumes the same permit through a rejection path,
records one bounded failure code, and emits `RuntimeAdmissionRejected`. The
physical service must already have restored every transferred frame before the
semantic rejection can commit.

## X86 Broker

`NativeRuntimeAdmissionBroker` joins a prepared semantic permit to
`NativeAddressSpaceService`. It verifies the supplied Capsule against the Image
bound to the permit, constructs the trusted `AgentCallContext`, and executes the
physical transaction.

Success commits the permit and queue entry. A semantic commit failure after
physical registration removes the still-prepared CPU from the native runtime,
clears its complete address space, and restores the frame pool. A physical
failure maps its stage to the bounded semantic rejection code only after pool
rollback evidence succeeds.

These broker mutations are architecture consequences of the two explicit
semantic Events. The frame identity remains architecture-private.

## Reference Ring-3 Flow

After the initial six native contexts terminate and return 66 frames:

1. Agent 10 and Agent 11 are registered, launched, and accepted without queueing;
2. Agent 12 is launched as an Admission Supervisor with task authority and a
   separate root-scoped `Delegate` Capability;
3. one bootstrapped physical admission starts Agent 12;
4. Agent 12 executes `DescribeContext`, two `RequestRuntimeAdmission` calls,
   `SubmitTaskResult`, and `CompleteTask`;
5. Agent 12 is verified and its eleven frames return to the pool;
6. the x86 broker consumes requests 1 and 2 in FIFO order;
7. Agent 10 and Agent 11 become admitted and queued through atomic commits;
8. both Workers execute, complete, verify, and return their 22 frames.

The direct physical start for Agent 12 is the fixed bootstrap boundary for the
admission control plane. Future userspace startup policy can replace that one
boot-owned transition without changing the request protocol.

## Deterministic Reference Profile

| Evidence | Count |
| --- | ---: |
| Registered Agents | 12 |
| Native ring-3 completions | 9 |
| Kernel-selected dispatches | 29 |
| Admission Supervisor Agent Calls | 5 |
| Admission Supervisor address-space switches | 10 |
| Runtime Service Worker Agent Calls | 6 |
| Runtime Service Worker address-space switches | 12 |
| Physical quantum expiries | 13 |
| Runtime admission requests | 2 |
| Runtime admission commits | 2 |
| Address-space cancellation cycles | 1 |
| Frames restored by admission cancellation | 11 |
| Terminal address-space reclamations | 9 |
| Terminal private-frame returns | 99 |
| Final zeroed private frame pool | 66 |
| Capabilities | 23 |
| Intents | 10 |
| Tasks | 10 |
| Ordered kernel Events | 264 |

## QEMU Evidence

The reference boot adds these exact-once proofs:

```text
AGENT_KERNEL_NATIVE_RUNTIME_ADMISSION_REQUEST_OK
AGENT_KERNEL_NATIVE_RUNTIME_ADMISSION_SUPERVISOR_OK
AGENT_KERNEL_NATIVE_RUNTIME_ADMISSION_COMMIT_OK
```

Debug and release runs must preserve all existing markers, emit exactly 264
ordered Events, end at `event[264] driver_invocation_completed`, and restore all
private and shared runtime frames.

## Validation Evidence

The implementation passed these contract layers:

- five core protocol tests covering Supervisor identity and operation authority,
  bound request identity, read-only permit preparation, atomic commit and
  queueing, duplicate/capacity atomicity, stale permit rejection, and bounded
  physical rejection;
- one facade protocol test covering the complete request, prepare, and commit
  boundary;
- three Agent Call 27 tests covering strict decode, canonical reply registers,
  authenticated native transcript, and malformed reserved registers;
- complete workspace tests and host Supervisor execution;
- `no_std` library checks, the freestanding `x86_64-unknown-none` binary check,
  workspace Clippy, and bare-metal Clippy with warnings denied;
- strict debug and release QEMU runs with 264 ordered Events and exact marker
  counts.

The release ELF contains exactly one 296-byte Admission Supervisor Capsule.
Its SHA-256 is
`8a86b7fcb03467cdc66c3f3730ef7c87bf9b2549610768e1c9c81624004d0f42`.
The embedded 264-byte code matches the fresh assembly artifact with SHA-256
`4c5fa49da402d0287d595f73f45afa7e5c7e72fc88d56f1beea55e423019e612`.
The symbol table fixes the five Agent Call return offsets at 44, 82, 169, 233,
and 262 bytes. Each new release marker occurs exactly once in the ELF; the
Agent Call request marker occurs exactly twice in the QEMU transcript.

## Deferred Work

- keeping the Admission Supervisor resident while target Agents execute;
- target cancellation while a request is pending or a CPU is active;
- dynamic private page-table hierarchy growth;
- persistent admission queues larger than the fixed Task store;
- SMP synchronization, PCID lifecycle, and hardware TLB shootdown.
