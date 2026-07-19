# Capability Store Compaction V1 Design

## Status

Implemented, validated, and published on 2026-07-19.

## Purpose

Capability revocation removes authority but retains the record in the sparse,
fixed-capacity Capability Store. Long-running Supervisors eventually exhaust
all slots even when revoked authority has no remaining live consumer.

This milestone adds authenticated retirement for one revoked Capability leaf.
Single-record retirement matches the store's existing sparse slots, permits
deterministic hole reuse, preserves monotonic IDs, and keeps capability lineage
in ordered Events.

## Eligibility And Ordering

`compact_capability(actor, authority, target)` accepts exactly one target.

- The target must exist and have `revoked == true`.
- No retained Capability may name the target as its direct parent.
- Descendant trees therefore retire leaf first and root last.
- Clearing a slot does not move any retained record.
- `next_capability` remains monotonic; retired IDs never re-enter the store.
- `capability(target)` returns `CapabilityNotFound` after compaction.
- Allocation continues to use the first empty physical slot and issues the
  next monotonic ID.

An unrevoked child remains an explicit retained record even when an ancestor
has already made its chain unusable. It must be revoked and retired separately.

## Authority

The actor must be active and have a launched `Supervisor` entry. The authority
must be an active, root-scoped Capability carrying `Rollback`.

For a target on an active Resource, the authority must match that Resource
exactly. For a target on a retired Resource, the authority may belong to an
active ancestor Resource. Resource ancestry is immutable, bounded by the
Resource Store capacity, and checked before mutation. This rule lets a resident
Supervisor finish metadata cleanup after a child Resource has entered its
terminal state without granting ancestor authority over live child Resources.

The target cannot authorize its own compaction because eligibility requires it
to be revoked.

## Active References

The target cannot leave the active store while any live kernel object still
depends on its identity. Preflight rejects references from:

- any retained child Capability;
- any Task whose `delegated_capability` is the target;
- any Agent Entry whose execution authority is the target;
- any Runtime Admission record whose `authority` is the target;
- any `Pending` or `Received` Message payload carrying the target.

Task and Runtime Admission compaction must run before Capability compaction
when those active stores retain the target. Agent Entry retirement remains a
later prerequisite for launch authorities.

Acknowledged Messages and stored Action, Observation, Checkpoint, Namespace,
Resource, Agent Image, Fault, Driver, and Event evidence are historical. Their
Capability IDs remain queryable in their own records and Events and do not
authorize later operations. The current Namespace object enum has no
Capability variant.

## Atomic Mutation

Compaction performs a read-only preflight:

1. authenticate the launched Supervisor;
2. locate the target and require explicit revocation;
3. validate exact-Resource or retired-descendant `Rollback` authority;
4. reject every active reference class;
5. reserve one Event slot.

After preflight, the kernel clears exactly one sparse slot and records one
`CapabilityCompacted` Event. Every failure leaves the Capability Store, Event
log, lengths, and ID counter unchanged.

Capability operations currently have no prepare/commit permit. Clearing a slot
introduces no Capability generation in V1. Runtime Admission commits already
revalidate their stored authority against current Capability state.

## Receipt

`CapabilityCompaction` is a copyable value exposing the retired Capability ID
through `capability()`. It contains no pointer or mutable store reference.

The public inspection boundary adds:

- `capability_capacity()` for fixed slot capacity;
- `capability_count()` for occupied sparse slots;
- the existing `capability(id)` lookup for exact identity checks.

## Event Contract

`CapabilityCompacted` records:

- the compacting Supervisor;
- the target Capability ID in `capability`;
- the compaction authority in `source_capability`;
- `Rollback` as the operation;
- the target's Resource, Agent, operation set, and optional Task scope;
- the next global Event sequence.

The original parent is preserved by the earlier `CapabilityGranted` or
`CapabilityDerived` Event. Revocation and derivation Events remain unchanged.

## Agent Call 32

The native ABI adds:

```text
CompactCapability = 32
```

Request payload:

| Register | Value |
| --- | --- |
| `r10` | compaction authority Capability ID |
| `r11` | revoked target Capability ID |
| `r12-r15`, `rbp` | zero |

The authenticated Agent, current Task, Image, and nonce remain in `rsi`,
`rdi`, `r8`, and `r9`.

Successful reply payload:

| Register | Value |
| --- | --- |
| `r10` | compacted Capability ID |
| `r11-r15`, `rbp` | zero |

Malformed, unauthenticated, unrevoked, referenced, unauthorized, and Event
capacity failures fail closed.

## X86 Proof

The x86 profile expands the Capability Store from 25 to 26 slots. The resident
Admission Supervisor uses Capability 23 to derive Capability 26, filling the
store, and then revokes it. Agent Call 32 retires two leaves:

- Capability 14 on retired child Resource 3, authorized from active ancestor
  Resource 1;
- Capability 26 on active Resource 1, authorized by exact scope.

The Supervisor then derives Capability 27 with `Delegate` and Capability 28
with `Rollback` into the reclaimed physical slots. Both grants remain strict
subsets of Capability 23. IDs 14 and 26 stay absent, IDs 27 and 28 prove
monotonic allocation, and the store returns to 26 occupied slots.

The resulting Supervisor Capsule is 2,164 bytes with 2,132 bytes of executable
code and SHA-256 digest
`016518b46a19fb533f6d1c95cbe0a950b34fe2872083389d4b68d7b95fd9a4bc`.
Its transcript contains 24 Agent Calls and 48 Agent/kernel address-space
switches. Events 331 through 336 are `CapabilityDerived`,
`CapabilityRevoked`, two `CapabilityCompacted` records, and two further
`CapabilityDerived` records. The complete boot proof contains 356 ordered
Events.

Expected proof markers:

```text
AGENT_KERNEL_AGENT_CALL_CAPABILITY_COMPACTION_OK
AGENT_KERNEL_NATIVE_CAPABILITY_COMPACTION_OK
```

## Failure Rules

- Unknown or zero target returns `CapabilityNotFound`.
- An unrevoked target returns `CapabilityCompactionNotReady`.
- A retained child or active object reference returns
  `CapabilityCompactionReferenced`.
- Worker callers fail the Supervisor-entry check.
- Missing, revoked, foreign, task-scoped, attenuated, or wrongly scoped
  authority returns the existing Capability or Resource error.
- Event exhaustion returns `EventLogFull` before mutation.
- Slot clearing, retained-record stability, historical lineage, and monotonic
  IDs are mandatory postconditions.

## Deferred Work

- Agent Entry retirement for completed launch identities;
- cleanup authority for a retired root Resource with no active ancestor;
- batch credentials spanning unrelated Resource trees;
- Message, Waiter, Fault, Resource, and Event retention policies;
- durable audit export and replay checkpoints.
