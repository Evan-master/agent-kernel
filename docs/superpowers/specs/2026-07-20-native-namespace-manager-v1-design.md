# Native Namespace Manager V1 Design

Status: Implemented, verified, and published

Publication: public `main`, implementation commit `1154c9e`

## Objective

Expose the existing AgentOS object Namespace through the native ring-3 Agent
Call boundary and recover its fixed-capacity records deterministically. An
authorized Agent can bind, resolve, rebind, and retire typed object names while
the kernel preserves monotonic identities, complete audit evidence, and stable
dense Store order.

This closes two lifecycle gaps. Native Agents gain direct Namespace control,
and a live Namespace reference can be removed before Message, Task, Agent,
Resource, or MemoryCell record retirement.

## Ownership Boundaries

- `agent-kernel-core` owns Namespace records, object validation, authorization,
  stable dense retirement, receipts, and Events.
- `agent-kernel` exposes the four operations through syscall-style methods.
- `agent-kernel-boot` makes Namespace capacity an explicit boot-profile
  parameter while preserving zero-capacity defaults.
- `agent-kernel-x86_64` owns Agent Calls 44 through 47, packed object encoding,
  strict register contracts, native postcondition checks, and ring-3 evidence.

The x86 reference profile boots Resource 1 as its root Workspace. The explicit
Port Endpoint descriptor and `Delegate` Capability allow that Workspace to
also host the boot Driver control-plane endpoint without adding ambient access.

No host path, string parser, filesystem object, allocator, or ambient authority
enters the native model.

## Core Retirement Protocol

Core adds:

```text
retire_namespace_entry(actor, authority, target)
```

Validation order is deterministic:

1. authenticate an active Agent;
2. resolve the target Namespace Entry;
3. require `Rollback` authority on the entry's Workspace Resource;
4. reserve one Event slot;
5. remove the target with a stable dense shift and clear the vacated tail;
6. append one `NamespaceEntryRetired` Event and return the complete record.

Every failure occurs before mutation. `next_namespace_entry` stays unchanged,
so a returned slot receives a fresh monotonic identity on later binding.

Namespace Entry identities have no persistent live references outside the
Namespace Store. Historical Events remain valid because IDs never alias.

## Receipt And Event

`NamespaceEntryRetirement` contains the complete removed record, actor, and
retirement authority. The record preserves owner, Workspace, original binding
Capability, key, typed object, and revision.

The retirement Event records:

- `kind = NamespaceEntryRetired`;
- `agent = actor`;
- `resource = record.namespace`;
- `capability = retirement authority`;
- `namespace_entry = record.id`;
- `namespace_key = record.key`;
- `namespace_object = record.object`;
- `operation = Rollback`;
- `target_agent = record.owner`.

The Event archive tag is 87. Existing tags and archive format version 1 remain
stable.

## Packed Namespace Object

The native ABI uses one canonical object word:

```text
word = (object_id << 3) | object_tag
```

Tags are:

| Tag | Object |
| ---: | --- |
| 1 | Agent |
| 2 | Resource |
| 3 | Task |
| 4 | Message |
| 5 | MemoryCell |

Tag zero, tags 6 and 7, object ID zero, and IDs above 61 bits are rejected.
Encoding and decoding round-trip every accepted word.

## Agent Calls 44 Through 47

### BindNamespaceEntry = 44

Request:

| Register | Value |
| --- | --- |
| `r10` | `Act` Capability on the Workspace |
| `r11` | Workspace Resource ID |
| `r12` | Namespace key |
| `r13` | packed Namespace object |
| `r14-r15`, `rbp` | zero |

### ResolveNamespaceEntry = 45

Request:

| Register | Value |
| --- | --- |
| `r10` | `Observe` Capability on the Workspace |
| `r11` | Workspace Resource ID |
| `r12` | Namespace key |
| `r13-r15`, `rbp` | zero |

### RebindNamespaceEntry = 46

Request:

| Register | Value |
| --- | --- |
| `r10` | `Act` Capability on the Workspace |
| `r11` | Namespace Entry ID |
| `r12` | packed replacement object |
| `r13-r15`, `rbp` | zero |

### RetireNamespaceEntry = 47

Request:

| Register | Value |
| --- | --- |
| `r10` | `Rollback` Capability on the Workspace |
| `r11` | Namespace Entry ID |
| `r12-r15`, `rbp` | zero |

Every successful operation returns the resulting or removed record in the same
canonical layout:

| Register | Value |
| --- | --- |
| `r10` | Namespace Entry ID |
| `r11` | owner Agent ID |
| `r12` | Workspace Resource ID |
| `r13` | original binding Capability ID |
| `r14` | Namespace key |
| `r15` | packed Namespace object |
| `rbp` | revision |

The scheduler-owned common reply fields remain unchanged.

## X86 Full-Capacity Proof

`BootedKernel` gains a final `NAMESPACE_ENTRIES` generic parameter with a zero
default. The x86 profile provisions one slot. Resource Manager Agent 8 holds
Capability 12 on Workspace 1 with `Observe`, `Act`, `Rollback`, and `Delegate`.

After MemoryCell 2's physical page is released, the resident Resource Manager
performs:

1. bind Entry 1 at key `0x4e53_0001` to MemoryCell 2;
2. resolve the key and receive the complete Entry 1 descriptor;
3. rebind Entry 1 to Resource Manager Agent 8, producing revision 2;
4. retire Entry 1 through `Rollback` authority;
5. bind Entry 2 at key `0x4e53_0002` to root Workspace 1 in the returned slot.

The inserted Event sequence is:

```text
186 namespace_entry_bound
187 namespace_entry_resolved
188 namespace_entry_rebound
189 namespace_entry_retired
190 namespace_entry_bound
```

Required terminal evidence:

- Namespace Store capacity and occupancy are both one;
- Entry 1 no longer resolves and Entry 2 occupies the returned physical slot;
- Entry 2 preserves owner 8, Workspace 1, Capability 12, key `0x4e53_0002`,
  Resource object 1, and revision 1;
- Resource Manager executes 39 Agent Calls and 78 address-space switches;
- live Event capacity is 362 and reaches full occupancy before archive commit;
- the retained Event 1 through 64 digest remains unchanged;
- archived and live iterators form the exact Event 1 through 396 transcript;
- final live Event occupancy is 332 and next Event sequence is 397;
- debug and release QEMU agree on Events, markers, Capsule bytes, digest, and
  return offsets.

## Frozen Resource Manager Artifact

- machine code: 3,816 bytes;
- complete kind-4 Capsule: 3,848 bytes;
- SHA-256:
  `8914b2dc4f1a1c5d93d6d7315ee5e289579fdbeee543b70f121abcce2a8bced6`;
- Agent Calls / address-space switches: 39 / 78;
- return offsets: `45, 86, 163, 236, 310, 390, 463, 539, 626, 710, 828,
  912, 996, 1080, 1200, 1320, 1443, 1569, 1643, 1762, 1855, 1945,
  2075, 2205, 2332, 2465, 2598, 2675, 2821, 2949, 3026, 3172, 3266,
  3394, 3471, 3617, 3731, 3805, 3814`;
- Namespace call markers: bind 2, resolve 1, rebind 1, retirement 1;
- Event archive checkpoint digest for Events 1 through 64:
  `6c3d502efb373196813fd512704a931e41bb5351834ee884581dce3d97965615`.

## Failure Rules

- inactive or unknown caller: existing Agent errors;
- unknown entry: `NamespaceEntryNotFound`;
- non-Workspace namespace: `ResourceKindMismatch`;
- missing, revoked, foreign, task-scoped, attenuated, or wrong-operation
  authority: existing authorization errors;
- duplicate key or missing object: existing Namespace/object lookup errors;
- full Namespace Store: `NamespaceEntryStoreFull`;
- malformed packed object or non-zero reserved register: ABI rejection;
- Event exhaustion: `EventLogFull` with every Store unchanged.

## Verification

Verified on 2026-07-20 with:

- focused Core, boot, facade, and x86 ABI contracts;
- the complete workspace test suite and Supervisor simulation;
- `rustfmt`, strict Clippy, Core/facade `no_std`, and the
  `x86_64-unknown-none` bare-metal target;
- debug and release QEMU boots through `SUPERVISOR_HANDOFF_READY` with the exact
  Event 1 through 396 transcript;
- independent assembly of both native manager images and byte-for-byte checks
  against their Rust Capsule arrays;
- exactly one Resource Manager Capsule and one Admission Supervisor Capsule in
  the release ELF.

## Deferred Work

- hierarchical Namespace Resources and delegated mount views;
- immutable compare-and-rebind generations for optimistic concurrency;
- batch retirement by owner, Workspace, or object;
- durable signed Namespace snapshots;
- architecture-neutral user-memory transfer for symbolic keys wider than one
  machine word.
