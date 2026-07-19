# Task Store Compaction V1 Design

## Status

Implemented and validated on 2026-07-19.

## Purpose

The fixed-capacity Task Store retains `Verified` and `Cancelled` records
indefinitely. Their ordered lifecycle already exists in the Event log, so
retaining every terminal Task in the active store eventually prevents a
resident Supervisor from creating more work.

This milestone adds authenticated compaction for a contiguous terminal Task
prefix. The operation returns active capacity, preserves monotonic Task IDs,
keeps Intent binding evidence, invalidates stale dispatch permits, and emits
one deterministic Event for each retired record.

## Active And Historical State

The Task array is the active scheduling and lifecycle store. The Event log is
the immutable history.

- `Verified` and `Cancelled` are eligible terminal states.
- Every other Task state remains active.
- A `Verified` Task must reference a `Fulfilled` Intent.
- A `Cancelled` Task must reference a `Cancelled` Intent.
- Compaction removes one nonempty contiguous prefix ending at an explicit
  `TaskId`.
- Remaining records preserve their order and complete record bytes.
- `next_task` remains monotonic; retired IDs never re-enter the active store.
- `task(id)` returns `TaskNotFound` after compaction.

Prefix-only removal keeps fixed storage compact and preserves visible FIFO
ordering without holes.

## Authority

The call accepts an actor, a root-scoped Capability, and the final Task ID in
the selected prefix.

1. The actor must be active and have a launched `Supervisor` entry.
2. Every selected Task must satisfy terminal and reference-readiness checks.
3. The Capability must grant `Rollback` over every selected Task Resource.
4. Existing chain, revocation, identity, Resource, and scope validation remains
   authoritative.

`Rollback` represents destructive retirement of active lifecycle state. The
compaction Event records the exact actor and Capability.

## Reference Readiness

A selected Task cannot leave the active store while a live kernel object still
depends on it. Preflight rejects a Task referenced by:

- any Run Queue entry;
- any Agent execution context;
- any active Signal or Mailbox waiter;
- any Runtime Admission record, including terminal records awaiting their own
  compaction;
- any Namespace entry bound to that Task;
- any `Pending` or `Received` Message payload.

Acknowledged Messages, inactive Waiters, Fault records, Events, task-scoped
Capabilities, and Agent launch entries retain historical Task IDs. Their
operations always revalidate the active Task Store before Task mutation, so a
retired ID cannot authorize new Task work. Runtime Admission compaction must
precede Task compaction whenever the two stores share an ID.

## Cancellation Queue Cleanup

Cancelling an `Accepted` Task may currently leave its queued entry behind.
`cancel_task` will remove every queue entry for the selected Task inside the
same preflighted transaction. The existing Task and Intent cancellation Events
remain the audit consequence.

## Atomic Mutation

`compact_task_prefix(actor, authority, through)` performs a read-only preflight:

- validate the launched Supervisor actor;
- locate `through` and derive the selected prefix;
- validate terminal Task and Intent states;
- validate `Rollback` authority for every Resource;
- validate every active reference class;
- reserve one Event slot per selected Task.

After preflight, the kernel copies the fixed array, shifts retained records to
index zero, clears vacated slots, updates the active length, and advances
`task_generation` once. Compaction Events are then appended in original Task
order. Every failed preflight leaves Tasks, queue state, Events, generation,
and ID counters unchanged.

## Receipt

`TaskCompaction` is a copyable value with three getters:

| Field | Meaning |
| --- | --- |
| `first` | first retired Task ID |
| `through` | final retired Task ID |
| `count` | number of retired active records |

The receipt contains no pointer or mutable store reference.

## Event Contract

Each retired record emits `TaskCompacted` with:

- compacting Supervisor and Capability;
- `Rollback` operation;
- original Task, Intent, Resource, assignee, result, run ticks, and last Fault;
- the next global Event sequence.

Existing creation, delegation, result, completion, verification,
cancellation, and Intent Events remain unchanged.

## Dispatch Permit Invalidation

`TaskDispatchPermit` gains a private Task generation. Preparation captures the
current generation; commit rejects a generation mismatch with
`TaskDispatchPermitStale`. Successful Task compaction advances the generation
once, invalidating permits prepared before array movement even when their Task
remains active.

Normal queue-head changes continue to return `TaskNotRunnable` through the
existing revalidation path.

## Agent Call 30

The native ABI adds:

```text
CompactTasks = 30
```

Request payload:

| Register | Value |
| --- | --- |
| `r10` | compaction Capability ID |
| `r11` | terminal prefix `through` Task ID |
| `r12-r15`, `rbp` | zero |

The authenticated Agent, current Task, Image, and nonce remain in `rsi`,
`rdi`, `r8`, and `r9`.

Successful reply payload:

| Register | Value |
| --- | --- |
| `r10` | first compacted Task ID |
| `r11` | final compacted Task ID |
| `r12` | compacted count |
| `r13-r15`, `rbp` | zero |

Malformed, unauthenticated, unauthorized, referenced, nonterminal, and Event
capacity failures fail closed.

## X86 Proof

The reference profile fills all 12 Task slots. After the first native runtime
phase, bootstrap verification finalizes Tasks 2 through 6; Task 1 is already
verified by the native Verifier. During the resident Admission Supervisor
session, Runtime Admission records 1 and 2 are released and compacted first.
The Supervisor then invokes Agent Call 30 with `Rollback` authority and Task 6
as the terminal boundary.

The reply must be `{ first: 1, through: 6, count: 6 }`. Tasks 7 through 12 stay
active in original order, the Supervisor keeps running in its existing address
space, and all subsequent release and verification paths use stable Task IDs.

Expected proof markers:

```text
AGENT_KERNEL_AGENT_CALL_TASK_COMPACTION_OK
AGENT_KERNEL_NATIVE_TASK_COMPACTION_OK
```

## Frozen Reference Evidence

The validated x86 reference profile freezes the following evidence:

| Evidence | Value |
| --- | ---: |
| Admission Supervisor Capsule code bytes | 1,294 |
| Admission Supervisor Capsule bytes | 1,326 |
| Admission Supervisor authenticated Agent Calls | 17 |
| Admission Supervisor Agent/kernel address-space switches | 34 |
| Task Store capacity before compaction | 12 |
| Compacted Task records | 6 |
| Active Task records after compaction | 6 |
| `TaskCompacted` Events | 6 |
| Ordered Events after Driver completion | 344 |

- Capsule code SHA-256:
  `719c0265579759c35ff3df32b3ffa15d2be0f53902473a3ece5b82cc07c378bb`
- Complete Capsule SHA-256:
  `3b27cd679d9a1ae0e15a28861b6703b3cfb483c2c350b158e002de6c4b9be311`
- Compacted Task IDs: 1 through 6.
- Retained active Task IDs: 7 through 12, in original order.
- Both Debug and Release strict QEMU runs reach
  `SUPERVISOR_HANDOFF_READY` with exactly 344 ordered Events.

## Failure Rules

- Unknown or zero `through` returns `TaskNotFound`.
- A nonterminal Task or inconsistent terminal Intent returns
  `TaskCompactionNotReady`.
- A live reference returns `TaskCompactionReferenced`.
- A Task permit prepared before successful compaction returns
  `TaskDispatchPermitStale` at commit.
- Worker callers fail the Supervisor-entry check.
- Missing, revoked, foreign, task-scoped, or attenuated authority returns the
  existing Capability error.
- Event exhaustion returns `EventLogFull` before mutation.
- Active ordering, vacated-slot clearing, and monotonic IDs are mandatory
  postconditions.

## Deferred Work

- Intent Store terminal compaction;
- Capability and Agent-entry lifecycle retirement;
- Message, Waiter, Fault, and Event retention policies;
- multi-Resource compaction credentials;
- durable audit export and replay checkpoints.
