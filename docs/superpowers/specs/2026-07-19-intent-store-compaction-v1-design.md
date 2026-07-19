# Intent Store Compaction V1 Design

## Status

Implemented and validated on 2026-07-19.

## Purpose

Task Store compaction returns Task capacity while leaving every terminal
Intent in the fixed-capacity Intent Store. Once all Intent slots are occupied,
a resident Supervisor cannot declare the next unit of work even when the
corresponding Tasks have left the active store.

This milestone adds authenticated compaction for a contiguous terminal Intent
prefix. It returns active capacity, preserves monotonic Intent IDs, requires
Task retirement first, and emits one deterministic Event for every retired
Intent.

## Active And Historical State

The Intent array is the active declaration and Task-binding store. Ordered
Events preserve lifecycle history.

- `Fulfilled` and `Cancelled` are eligible terminal states.
- `Declared` and `Bound` remain active.
- `Failed` remains ineligible until a kernel transition and matching Event
  contract exist for that status.
- Compaction removes one nonempty contiguous prefix ending at an explicit
  `IntentId`.
- Remaining records preserve their order and complete record bytes.
- `next_intent` remains monotonic; retired IDs never re-enter the active store.
- `intent(id)` returns `IntentNotFound` after compaction.

Prefix removal avoids holes in fixed storage and keeps declaration order
visible to deterministic callers.

## Authority

The operation accepts an actor, a root-scoped Capability, and the final Intent
ID in the selected prefix.

1. The actor must be active and have a launched `Supervisor` entry.
2. Every selected Intent must be eligible and free of active references.
3. The Capability must grant `Rollback` over every selected Intent Resource.
4. Existing identity, scope, chain, revocation, and Resource checks remain
   authoritative.

The compaction Event records the Supervisor and exact Capability used for the
destructive lifecycle transition.

## Reference Readiness

An Intent cannot leave the active store while another active object still
depends on it. Preflight rejects an Intent referenced by:

- any Task remaining in the active Task Store;
- any `Pending` or `Received` Message payload.

Task compaction must therefore precede Intent compaction for each lifecycle
pair. Acknowledged Messages, Agent launch entries, capability Events, fault
Events, Task Events, and all other ordered Events retain historical Intent IDs.
Agent admission and authorization paths revalidate current Task and Capability
objects and do not derive authority from an Agent Entry's historical Intent
field.

The current Namespace object model has no Intent variant, so it contributes no
Intent reference in this milestone.

## Atomic Mutation

`compact_intent_prefix(actor, authority, through)` performs a read-only
preflight:

- validate the launched Supervisor actor;
- locate `through` and derive the selected prefix;
- validate eligible terminal status for every selected record;
- validate `Rollback` authority for every selected Resource;
- validate every active reference class;
- reserve one Event slot per selected Intent.

After preflight, the kernel copies the fixed array, shifts retained records to
index zero, clears vacated slots, updates the active length, and appends
compaction Events in original declaration order. Every failed preflight leaves
Intents, Tasks, Messages, Events, lengths, and ID counters unchanged.

Intent operations currently use ID lookup and have no prepare/commit permit.
Array movement therefore introduces no Intent permit generation in V1.

## Receipt

`IntentCompaction` is a copyable value with three getters:

| Field | Meaning |
| --- | --- |
| `first` | first retired Intent ID |
| `through` | final retired Intent ID |
| `count` | number of retired active records |

The receipt contains no pointer or mutable store reference.

## Event Contract

Each retired record emits `IntentCompacted` with:

- compacting Supervisor and Capability;
- `Rollback` operation;
- original Intent ID, kind, Resource, owner, and verification requirement;
- the next global Event sequence.

Earlier declaration, binding, fulfillment, cancellation, Task, Capability,
Agent launch, and fault Events remain unchanged.

## Agent Call 31

The native ABI adds:

```text
CompactIntents = 31
```

Request payload:

| Register | Value |
| --- | --- |
| `r10` | compaction Capability ID |
| `r11` | terminal prefix `through` Intent ID |
| `r12-r15`, `rbp` | zero |

The authenticated Agent, current Task, Image, and nonce remain in `rsi`,
`rdi`, `r8`, and `r9`.

Successful reply payload:

| Register | Value |
| --- | --- |
| `r10` | first compacted Intent ID |
| `r11` | final compacted Intent ID |
| `r12` | compacted count |
| `r13-r15`, `rbp` | zero |

Malformed, unauthenticated, unauthorized, referenced, nonterminal, and Event
capacity failures fail closed.

## X86 Proof

The reference profile fills all 12 Intent slots. Bootstrap verification makes
Intents 1 through 6 `Fulfilled`, and Agent Call 30 removes their Tasks from the
active Task Store. The resident Admission Supervisor then invokes Agent Call
31 with Capability 23 and Intent 6 as the terminal boundary.

The reply must be `{ first: 1, through: 6, count: 6 }`. Intents 7 through 12
stay active in original order, the Supervisor remains in its current address
space, and Tasks 7 through 12 retain valid Intent bindings.

Expected proof markers:

```text
AGENT_KERNEL_AGENT_CALL_INTENT_COMPACTION_OK
AGENT_KERNEL_NATIVE_INTENT_COMPACTION_OK
```

## Frozen Reference Evidence

The validated x86 reference profile freezes the following evidence:

| Evidence | Value |
| --- | ---: |
| Admission Supervisor Capsule code bytes | 1,414 |
| Admission Supervisor Capsule bytes | 1,446 |
| Admission Supervisor authenticated Agent Calls | 18 |
| Admission Supervisor Agent/kernel address-space switches | 36 |
| Intent Store capacity before compaction | 12 |
| Compacted Intent records | 6 |
| Active Intent records after compaction | 6 |
| `IntentCompacted` Events | 6 |
| Ordered Events after Driver completion | 350 |

- Capsule code SHA-256:
  `865a9b3cf007b5ac065a545f5e7bc60dae150b4bb92d082dc71000742e4154b7`
- Complete Capsule SHA-256:
  `0988accada257e1e75e4b9adb4c645933b47d6fa05d9e3e9c1fdaaed52c12993`
- Return offsets:
  `44, 82, 169, 247, 358, 395, 506, 572, 659, 737, 848, 885, 996, 1059, 1179, 1299, 1383, 1412`.
- Compacted Intent IDs: 1 through 6.
- Retained active Intent IDs: 7 through 12, in original order.
- Reassembled source and the checked-in static Capsule match byte for byte.
- The Release kernel ELF contains exactly one complete Supervisor Capsule.
- Debug and Release strict QEMU runs reach `SUPERVISOR_HANDOFF_READY` with
  exactly 350 ordered Events.

## Failure Rules

- Unknown or zero `through` returns `IntentNotFound`.
- `Declared`, `Bound`, or `Failed` returns `IntentCompactionNotReady`.
- An active Task or Message reference returns `IntentCompactionReferenced`.
- Worker callers fail the Supervisor-entry check.
- Missing, revoked, foreign, task-scoped, or attenuated authority returns the
  existing Capability error.
- Event exhaustion returns `EventLogFull` before mutation.
- Active ordering, vacated-slot clearing, and monotonic IDs are mandatory
  postconditions.

## Deferred Work

- an explicit Intent failure transition and Event;
- Capability and Agent-entry lifecycle retirement;
- Message, Waiter, Fault, and Event retention policies;
- multi-Resource compaction credentials;
- durable audit export and replay checkpoints.
