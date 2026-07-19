# Runtime Admission Compaction V1 Design

## Status

Implemented and validated on 2026-07-19.

## Purpose

Runtime Admission records currently remain in the active fixed-capacity store
after `Rejected` or `Released`. Their complete lifecycle stays available in the
ordered Event log, yet every terminal record continues consuming an active
queue slot. A resident Supervisor therefore reaches `RuntimeAdmissionStoreFull`
after a bounded number of batches even when all physical and semantic work has
finished.

This milestone adds an authenticated, auditable compaction operation for a
contiguous terminal prefix. It returns active capacity, preserves monotonic
Admission IDs, invalidates stale permits, and keeps one deterministic Event for
every retired record.

## Active And Historical State

The Runtime Admission array is the active queue. The Event log is the immutable
lifecycle history.

- `Requested` and `Admitted` records are active and cannot be compacted.
- `Rejected` and `Released` records are terminal and can be compacted.
- Compaction removes only a contiguous prefix ending at an explicit
  `RuntimeAdmissionId`.
- Remaining records preserve FIFO order and complete record bytes.
- `next_runtime_admission` keeps increasing; compacted IDs are never reused.
- `runtime_admission(id)` returns `RuntimeAdmissionNotFound` after that ID leaves
  the active store.

Restricting removal to a prefix preserves the queue's visible ordering and
prevents holes in fixed-capacity storage.

## Authority

The operation accepts a trusted actor identity, one Capability, and the final
Admission ID in the prefix.

1. The actor must have a launched Supervisor entry.
2. Every selected record must be terminal.
3. The supplied Capability must grant the actor `Delegate` authority over every
   selected record's Resource.
4. Capability-chain revocation and scope checks use the existing authorization
   path.

One root-scoped Supervisor Capability can therefore compact a mixed-requester
terminal prefix when it covers every Resource. The Event identifies the actor
that performed compaction and the exact Capability used.

## Atomic Mutation

`compact_runtime_admission_prefix(actor, authority, through)` performs a full
read-only preflight:

- locate `through` in the active store;
- derive the nonempty prefix length;
- validate Supervisor identity and every terminal status;
- validate `Delegate` authority for every record;
- reserve one Event slot per selected record.

After preflight, the kernel copies the current fixed array, shifts the remaining
records toward index zero, clears every vacated slot, updates the active length,
and advances `runtime_admission_generation` once. It then appends the prepared
compaction Events in original FIFO order.

Every preflight failure leaves records, length, generation, IDs, and Events
unchanged.

## Receipt

The operation returns a copyable `RuntimeAdmissionCompaction` receipt:

| Field | Meaning |
| --- | --- |
| `first` | first compacted Admission ID |
| `through` | final compacted Admission ID supplied by the caller |
| `count` | number of active records retired |

The receipt carries no mutable store reference and no userspace pointer.

## Event Contract

Each removed record emits `RuntimeAdmissionCompacted` with:

- `agent`: compacting Supervisor;
- `capability`: Capability used for compaction;
- `operation`: `Delegate`;
- original Resource, Task, target Agent, Agent Image, and Runtime Admission ID.

Earlier request, admission, rejection, release, Task, and physical proof Events
remain unchanged. Replay can therefore establish both the terminal transition
and the later removal from the active queue.

## Permit Invalidation

Runtime Admission preparation and release permits share
`runtime_admission_generation`. A successful compaction advances that value
once. Any permit prepared before compaction becomes stale, including a permit
for a record that remains active after the shifted prefix.

This closes an ABA path where a permit could otherwise retain an old array
position after compaction.

## Agent Call 29

The native ABI adds:

```text
CompactRuntimeAdmissions = 29
```

Request payload:

| Register | Value |
| --- | --- |
| `r10` | compaction Capability ID |
| `r11` | terminal prefix `through` Admission ID |
| `r12-r15`, `rbp` | zero |

The common authenticated context remains in `rsi`, `rdi`, `r8`, and `r9`.

Successful reply payload:

| Register | Value |
| --- | --- |
| `r10` | first compacted Admission ID |
| `r11` | final compacted Admission ID |
| `r12` | compacted count |
| `r13-r15`, `rbp` | zero |

Malformed, zero, reserved, unauthenticated, unauthorized, nonterminal, and
capacity-failure paths fail closed.

## Resident X86 Proof

The existing ring-3 Admission Supervisor keeps one CPU and address space across
two Worker batches. After acknowledging the fourth completion notification, it
invokes Agent Call 29 with Capability 23 and Admission ID 2.

At that point:

- Admissions 1 and 2 are `Released`;
- Admissions 3 and 4 are `Admitted`;
- the active store contains all four records in FIFO order;
- the Supervisor is running in its original address space.

The call must return `{ first: 1, through: 2, count: 2 }`. The active store then
contains only Admissions 3 and 4. The Supervisor submits its result and
completes; terminal physical reclamation releases Admissions 3 and 4 from their
new indices.

## Deterministic Evidence

The reference profile changes by one Agent Call and two Events:

| Evidence | Count |
| --- | ---: |
| Supervisor Agent Calls | 16 |
| Supervisor address-space switches | 32 |
| Runtime Admission requests | 4 |
| Runtime Admission releases | 4 |
| Runtime Admission compactions | 2 |
| Retained terminal Runtime Admission records | 2 |
| Ordered kernel Events | 328 |

All Agent, Task, dispatch, preemption, notification, frame-return, and final
pool counts remain unchanged.

New proof markers:

```text
AGENT_KERNEL_AGENT_CALL_RUNTIME_ADMISSION_COMPACTION_OK
AGENT_KERNEL_NATIVE_RUNTIME_ADMISSION_COMPACTION_OK
```

## Failure Rules

- Unknown or zero `through` returns `RuntimeAdmissionNotFound`.
- Any selected `Requested` or `Admitted` record returns
  `RuntimeAdmissionCompactionNotReady`.
- Worker callers fail the Supervisor-entry check.
- Missing, revoked, foreign, task-scoped, or attenuated authority returns the
  existing Capability error.
- Event exhaustion returns `EventLogFull` before mutation.
- A successful compaction makes all earlier admission and release permits
  stale.
- Active ordering, vacated-slot clearing, and ID monotonicity are mandatory
  postconditions.

## Frozen Binary Evidence

- Supervisor Capsule: 1,206 bytes.
- Supervisor executable code: 1,174 bytes.
- Capsule SHA-256:
  `7f39ab25f4d01de012556befc42b19f991e4ec60e0cacee464eb2b33d8908b4b`.
- Code SHA-256:
  `a6a120bb46b30988ea9fa4b160035bcc32670f6f8aaeb624de5854cda4ace0b7`.
- Return offsets:
  `[44, 82, 169, 247, 358, 395, 506, 572, 659, 737, 848, 885, 996, 1059, 1143, 1172]`.
- Strict Debug and Release QEMU each completed all 328 ordered Events and
  reached `SUPERVISOR_HANDOFF_READY`.
- Full workspace tests, the host Supervisor run, formatting, shell validation,
  `no_std` checks, and the freestanding x86_64 check passed.
- Workspace and freestanding Clippy passed with warnings denied after retaining
  the repository's structural allowance for pre-existing
  `too_many_arguments` sites.
- The Release ELF contains one exact Capsule occurrence and one exact code
  occurrence; the executable bytes match the independently assembled `.text`
  section byte for byte.

## Deferred Work

- a dedicated Runtime Admission capacity generic independent from `TASKS`;
- an active queue capacity larger than the Task store;
- Event-log checkpointing and durable audit export;
- a userspace-selected retention policy across multiple requesters;
- an unbounded resident admission loop.
