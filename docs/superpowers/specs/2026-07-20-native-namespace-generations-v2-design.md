# Native Namespace Generations V2 Design

Status: Implemented and verified on 2026-07-20

## Objective

Add optimistic concurrency control to native Namespace mutations. An Agent can
replace or retire a binding only when its expected revision matches the live
record. Stale writers fail before every Store mutation and before Event
allocation, so retries remain deterministic and auditable.

This generation contract is the concurrency foundation for hierarchical
Namespace mounts. Existing unconditional Rebind and Retire operations remain
explicit force operations for callers that intentionally hold `Act` or
`Rollback` authority.

## Ownership Boundaries

- `agent-kernel-core` owns revision validation, mutation ordering, receipts,
  and failure atomicity.
- `agent-kernel` exposes syscall-style compare operations.
- `agent-kernel-x86_64` owns Agent Calls 48 and 49, strict register decoding,
  authenticated execution, canonical replies, and bare-metal evidence.
- The ring-3 Resource Manager proves both operations against the fixed-capacity
  Namespace Store.

No allocator, user pointer, host synchronization primitive, hidden lock, or
ambient authority enters the protocol.

## Core Protocol

Core adds:

```text
compare_and_rebind_namespace_entry(
    actor,
    authority,
    entry,
    expected_revision,
    replacement,
)

compare_and_retire_namespace_entry(
    actor,
    authority,
    entry,
    expected_revision,
)
```

Compare-and-rebind validates in this order:

1. authenticate an active Agent;
2. resolve the Namespace Entry;
3. require `Act` authority on its Workspace;
4. compare the live revision with the expected revision;
5. validate the replacement object;
6. reserve one Event slot;
7. replace the object, advance the revision once, and append one
   `NamespaceEntryRebound` Event;
8. return the complete resulting record.

Compare-and-retire validates in this order:

1. authenticate an active Agent;
2. resolve the Namespace Entry and its dense Store index;
3. require `Rollback` authority on its Workspace;
4. compare the live revision with the expected revision;
5. reserve one Event slot;
6. remove the record with a stable dense shift, clear the vacated tail, and
   append one `NamespaceEntryRetired` Event;
7. return the existing complete retirement receipt.

Revision mismatch returns `NamespaceRevisionMismatch`. Authorization is checked
before revision comparison, preventing revision probing through foreign
authority. Missing entries still return `NamespaceEntryNotFound`.

Every failure preserves Namespace records, Store order, Store occupancy,
`next_namespace_entry`, revisions, and the Event Log.

## Agent Calls 48 And 49

### CompareAndRebindNamespaceEntry = 48

| Register | Value |
| --- | --- |
| `r10` | `Act` Capability on the Workspace |
| `r11` | Namespace Entry ID |
| `r12` | expected revision, non-zero |
| `r13` | packed replacement Namespace object |
| `r14-r15`, `rbp` | zero |

### CompareAndRetireNamespaceEntry = 49

| Register | Value |
| --- | --- |
| `r10` | `Rollback` Capability on the Workspace |
| `r11` | Namespace Entry ID |
| `r12` | expected revision, non-zero |
| `r13-r15`, `rbp` | zero |

Both successful operations return the complete Namespace Entry record in the
Calls 44 through 47 canonical reply layout. The retire reply contains the
removed record.

Zero expected revision, malformed packed objects, and non-zero reserved
registers fail ABI decoding before Core execution.

## Bare-Metal Proof

The existing Resource Manager sequence binds Entry 1 at revision 1 and resolves
it. The generation-aware continuation then:

1. compare-and-rebinds Entry 1 at expected revision 1 to Agent 8, receiving
   revision 2;
2. compare-and-retires Entry 1 at expected revision 2;
3. binds Entry 2 in the returned physical slot at key `0x4e53_0002` to root
   Workspace 1.

The existing force Rebind and Retire proof steps are replaced in place, keeping
the exact Event window:

```text
event[188] namespace_entry_rebound
event[189] namespace_entry_retired
event[190] namespace_entry_bound
```

Required terminal evidence:

- Entry 1 is absent and Entry 2 occupies the single physical slot;
- Entry 2 has revision 1 and a fresh monotonic identity;
- Resource Manager executes 39 Agent Calls and 78 address-space switches;
- live Event capacity is 362 and reaches full occupancy before archive commit;
- archived and live Events form the exact Event 1 through 396 transcript;
- final live Event occupancy is 332 and next Event sequence is 397;
- debug and release QEMU agree on transcript, markers, Capsule bytes, digest,
  and return offsets.

## Verification Gates

- focused Core, facade, and x86 contract tests;
- stale revision, wrong authority, missing entry, invalid object, and Event
  exhaustion atomicity;
- workspace tests and Supervisor simulation;
- `rustfmt`, strict Clippy, Core/facade `no_std`, and bare-metal compile;
- debug and release QEMU transcript validation;
- independent Resource Manager assembly and exact Release ELF Capsule count.

## Verified Artifact

| Property | Value |
| --- | --- |
| Resource Manager Calls / switches | `39 / 78` |
| Capsule / code bytes | `3854 / 3822` |
| Capsule SHA-256 | `a34b39a50168bb128d4f4ca1d8a30b02c94087b1d47148215ca57e5e238be442` |
| Code SHA-256 | `8026e14861c0287ead6037ff72937389808de6ca8b8c16571475a16e4a5b0e80` |
| Release ELF Capsule / code occurrences | `1 / 1` |
| Transcript | Events `1..396` |

The independently assembled Capsule matches the Rust authority byte for byte.
Its 39 return offsets are:

```text
45, 86, 163, 236, 310, 390, 463, 539, 626, 710, 828, 912, 996,
1080, 1200, 1320, 1443, 1569, 1643, 1762, 1855, 1945, 2075, 2208,
2338, 2471, 2604, 2681, 2827, 2955, 3032, 3178, 3272, 3400, 3477,
3623, 3737, 3811, 3820
```

Core and facade `no_std` checks, strict Clippy, the complete Workspace test
suite, Supervisor simulation, bare-metal compile, debug QEMU, and release QEMU
all pass. Both QEMU profiles preserve the exact Event transcript and emit each
generation marker once.

## Deferred Work

- Workspace-to-Workspace mount records with explicit authority on every hop;
- bounded native path traversal followed by architecture-neutral user-memory
  transfer for longer symbolic paths;
- signed durable Namespace snapshots and distributed generation receipts.
