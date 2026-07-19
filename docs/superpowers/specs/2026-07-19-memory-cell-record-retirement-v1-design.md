# Memory Cell Record Retirement V1 Design

Status: Implemented, publication validation complete

## Objective

Recover bounded MemoryCell Store capacity after a Memory Resource has reached
`ResourceStatus::Retired` and its architecture-owned mapping and physical frame
binding have been removed. A launched Supervisor can retire the terminal
MemoryCell record, cleanup-revoke and compact its residual Capability, retire
the Resource record, and reuse every returned slot under fresh monotonic IDs.

This closes the semantic half of native memory reclamation. Page-table leaf
removal and frame zeroing already return physical capacity; this milestone
also returns the fixed Core records that describe the old allocation.

## Ownership Boundaries

- `agent-kernel-core` owns terminal Resource validation, cleanup authority,
  Core reference preflight, stable dense removal, receipts, and audit Events.
- `agent-kernel` exposes MemoryCell record retirement through the syscall-style
  facade.
- `agent-kernel-x86_64` owns Agent Call 43, strict register contracts, native
  binding preflight, audited replies, and ring-3 capacity-reuse evidence.
- Architecture-owned page, region, CPU, and frame-pool ledgers remain outside
  Core and must all report the target absent before the facade call.

## Core Retirement Protocol

Core adds:

```text
retire_memory_cell_record(actor, authority, target)
```

Validation order is deterministic:

1. authenticate a launched Supervisor;
2. resolve the target MemoryCell record;
3. resolve its Resource and require `ResourceKind::Memory` plus
   `ResourceStatus::Retired`;
4. authorize cleanup through an active ancestor `Rollback` Capability;
5. reject every retained Core reference;
6. reserve one Event slot;
7. remove the target with a stable dense shift and clear the vacated tail;
8. append one `MemoryCellRecordRetired` Event and return a receipt.

Every failure occurs before mutation. `next_memory_cell` remains unchanged.

## Reference Rules

Core rejects a target retained by `NamespaceObject::MemoryCell`. No other
persistent Core record contains a MemoryCell identity.

Historical Events are excluded. MemoryCell IDs come from a monotonic allocator,
so an archived identity cannot alias a later record.

Before invoking Core, the x86 handler additionally rejects a target present in:

- the current pending Agent CPU page or region ledger;
- any parked native Agent CPU context;
- any retained completed or faulted CPU memory ledger;
- any committed shared `RuntimeMemoryPool` binding.

Observations and reclamation logs remain historical evidence and do not resolve
back into live objects.

## Receipt And Audit Semantics

`MemoryCellRecordRetirement` contains the complete removed
`MemoryCellRecord`, actor, and authority. The record preserves creator,
last writer, four value words, and revision for external archival.

The Event records:

- `kind = MemoryCellRecordRetired`;
- `agent = actor`;
- `resource = removed record.resource`;
- `capability = cleanup authority`;
- `memory_cell = removed record.id`;
- `operation = Rollback`;
- `target_agent = removed record.last_writer`.

`MemoryCellRecordRetired` receives Event archive tag 86. All existing tags and
Event Archive format version 1 remain unchanged.

## Agent Call 43

The native ABI adds:

```text
RetireMemoryCellRecord = 43
```

Request payload:

| Register | Value |
| --- | --- |
| `r10` | active ancestor cleanup Capability ID |
| `r11` | terminal MemoryCell ID |
| `r12-r15`, `rbp` | zero |

Successful reply payload:

| Register | Value |
| --- | --- |
| `r10` | retired MemoryCell ID |
| `r11` | retired Memory Resource ID |
| `r12` | final revision |
| `r13-r15`, `rbp` | final value words 0 through 3 |

Creator and last-writer identities remain in the Core receipt and historical
audit chain. The reply keeps the descriptor and revision available without a
userspace pointer.

## X86 Full-Capacity Proof

The x86 profile keeps MemoryCell capacity at five, Resource capacity at seven,
Capability capacity at 26, and live Event capacity at 357. Resource Manager
leaves MemoryCell 2 bound to retired Memory Resource 4 with active Capability
16 after its physical page has already been removed and zeroed.

After the existing Event archive and Resource 3 reuse proof, the resident
Admission Supervisor performs this sequence:

1. retire MemoryCell record 2 through Agent Call 43;
2. cleanup-revoke and compact Capability 16;
3. retire Resource record 4;
4. create Memory Resource 9 under root Resource 1 with Capability 30 carrying
   `Observe`, `Act`, and `Rollback`;
5. allocate one private runtime page, creating MemoryCell 6 in the returned
   dense slot, and write proof value `0x4d454d43454c4c36`;
6. submit its result and complete; completion reclamation retires Resource 9,
   removes the page leaf, zeroes and returns the frame, and retains MemoryCell
   6 as terminal evidence.

The frozen Event tail is:

```text
363 memory_cell_record_retired
364 capability_revoked
365 capability_compacted
366 resource_record_retired
367 resource_created
368 capability_granted
369 memory_cell_created
370 task_result_submitted
371 resource_retired
372 task_completed
```

Required terminal evidence:

- MemoryCell IDs are `[1, 3, 4, 5, 6]` at full five-slot occupancy;
- Resource IDs are `[1, 2, 5, 6, 7, 8, 9]` at full seven-slot occupancy;
- MemoryCell 2, Resource 4, and Capability 16 no longer resolve;
- MemoryCell 6 records Resource 9, the Supervisor, the fixed page descriptor,
  revision 1, and the fresh monotonic identity;
- Resource 9 is terminal and Capability 30 remains active;
- the Supervisor executes 44 Agent Calls and 88 address-space switches;
- the retained Event 1 through 64 archive digest stays unchanged;
- archived and live iterators form one exact transcript through Event 391;
- final live Event occupancy is 327 and the next Event sequence is 392;
- debug and release QEMU agree on all Events, marker counts, reclamation
  evidence, Capsule bytes, digest, and return offsets.

The Admission Supervisor Capsule contains 4,082 code bytes and 4,114 total
bytes. Its SHA-256 digest is
`3acd53283d17e77952a5742b895b2f4b578ee768faf497bce070a86397c6cb42`.
The final eight return offsets are:

```text
3699, 3767, 3815, 3857, 3917, 3965, 4051, 4080
```

The code remains inside the fixed 4 KiB Capsule execution page. Operation 43
returns at offset 3699 and terminal completion returns at offset 4080.

## Failure Rules

- missing launched Supervisor: existing Agent Entry errors;
- non-Supervisor caller: `AgentEntryKindMismatch`;
- active backing Resource: `MemoryCellRecordRetirementNotReady`;
- retained Namespace reference: `MemoryCellRecordRetirementReferenced`;
- active x86 page, region, CPU, or frame-pool binding: fail closed before Core;
- missing target: `MemoryCellNotFound`;
- invalid, revoked, foreign, task-scoped, attenuated, or unrelated authority:
  existing Capability and Resource scope errors;
- Event exhaustion: `EventLogFull` with every Store unchanged.

## Deferred Work

- batch MemoryCell retirement for one terminal Resource subtree;
- durable encrypted receipt storage and retention policy;
- Namespace Entry retirement so named MemoryCells can complete the same path;
- architecture-neutral registration of external live-reference providers;
- cascading cleanup transactions spanning MemoryCell, Capability, and Resource
  records under one generation-bound permit.
