# Resource Record Retirement V1 Design

Status: Implemented and validated

## Objective

Recover bounded Resource Store capacity after a Resource has reached its
terminal lifecycle state. A launched Supervisor can revoke residual
Capabilities through active ancestor authority, compact those Capability
records, and then retire an unreferenced Resource record from the dense Store.

Resource IDs remain monotonic. Later creation may reuse the returned physical
slot, while every newly created Resource receives a fresh ID. Historical
Events retain old IDs without creating identity aliases.

## Ownership Boundaries

- `agent-kernel-core` owns terminal-state checks, cleanup authorization,
  complete non-Event reference validation, dense removal, receipts, and audit
  Events.
- `agent-kernel` exposes Capability cleanup revocation and Resource record
  retirement through syscall-style facade methods.
- `agent-kernel-x86_64` owns Agent Calls 41 and 42, strict register contracts,
  audited handlers, canonical replies, and ring-3 slot-reuse evidence.
- `agent-supervisor` may persist returned retirement receipts. It never gains
  direct access to Core arrays.

## Prerequisite Capability Cleanup

Resource creation installs an initial Capability that can remain active after
the Resource enters `ResourceStatus::Retired`. Existing derived-Capability
revocation requires direct parent authority, so an independent Supervisor may
have no valid path to revoke that initial record.

Core therefore adds:

```text
revoke_capability_for_cleanup(actor, authority, target)
```

The operation requires:

- a launched `Supervisor` Agent Entry for `actor`;
- an active, task-unscoped Capability owned by `actor` with `Rollback`;
- a target Capability whose Resource is already retired;
- scope on an active ancestor selected by the shared cleanup-authorization
  walk;
- one free Event slot.

Success marks only the target Capability revoked and appends one
`CapabilityRevoked` Event. The Event records the target, cleanup authority,
target Agent, Resource, operation set, optional Task scope, and
`Operation::Rollback`. Existing `compact_capability` then performs reference
validation and returns the sparse Capability slot.

## Resource Retirement Protocol

Core adds:

```text
retire_resource_record(actor, authority, target)
```

Validation order is deterministic:

1. authenticate a launched Supervisor;
2. resolve the target in the retained Resource prefix;
3. require `ResourceStatus::Retired`;
4. authorize cleanup through the target's active ancestor chain;
5. reject every retained non-Event reference;
6. reserve one Event slot;
7. remove the target with a stable dense shift and clear the vacated tail;
8. append one `ResourceRecordRetired` Event and return a receipt.

Every failure occurs before mutation. `next_resource` remains unchanged.

## Complete Reference Matrix

The preflight rejects a target retained by any of these Core-owned locations:

- another Resource's `parent`;
- Capability `resource`;
- Agent `management_resource`;
- Agent Entry `resource`;
- Agent Image `resource`;
- Intent, Task, Runtime Admission, Action, Observation, or Checkpoint
  `resource`;
- Message payload `resource`;
- Memory Cell `resource`;
- Namespace Entry `namespace` or `NamespaceObject::Resource`;
- Fault, Fault Handler, Fault Policy, or Waiter `resource`;
- Driver Endpoint, Driver Binding, Device Event, Driver Command, or Driver
  Invocation `resource`;
- the retained Event Archive checkpoint root.

Historical Events are excluded. Resource IDs come only from the monotonic
allocator, so an Event reference can never resolve to a later object.

## Receipt And Audit Semantics

`ResourceRecordRetirement` contains the complete removed `Resource`, actor,
and authority. The caller can hand that fixed-width receipt to an external
archive before discarding it.

The Event records:

- `kind = ResourceRecordRetired`;
- `agent = actor`;
- `resource = target`;
- `capability = cleanup authority`;
- `operation = Rollback`;
- `target_agent = Resource.owner` when present.

`ResourceRecordRetired` receives Event archive tag 85. All existing tags stay
unchanged, and Event Archive format version 1 remains unchanged.

## Agent Call 41

The native ABI adds:

```text
RetireResourceRecord = 41
```

Request payload:

| Register | Value |
| --- | --- |
| `r10` | active ancestor cleanup Capability ID |
| `r11` | retired Resource ID |
| `r12-r15`, `rbp` | zero |

Successful reply payload:

| Register | Value |
| --- | --- |
| `r10` | retired Resource ID |
| `r11` | stable Resource kind code |
| `r12` | parent Resource ID, or zero |
| `r13` | owner Agent ID, or zero |
| `r14-r15`, `rbp` | zero |

Kind codes preserve the existing native values: Workspace 1, Memory 2,
Service 3, Network 4, and Device 5. File and Process use reply-only codes 6
and 7; create requests continue to reject those legacy-facing kinds.

## Agent Call 42

The native ABI also adds:

```text
RevokeCapabilityForCleanup = 42
```

Request payload:

| Register | Value |
| --- | --- |
| `r10` | active ancestor cleanup Capability ID |
| `r11` | Capability ID scoped to a retired Resource |
| `r12-r15`, `rbp` | zero |

The reply returns the revoked Capability in `r10`, its Resource in `r11`, and
zero in every remaining payload register.

## X86 Full-Capacity Proof

The x86 profile keeps Resource capacity at seven, Capability capacity at 26,
and live Event capacity at 357. Resource Manager leaves Resource 3 retired,
with initial Capability 13 active and derived Capability 14 revoked.

The resident Admission Supervisor performs this terminal sequence:

Its root-scoped Capability 23 carries `Act`, `Delegate`, and `Rollback` for
admission, cleanup, and the fresh Resource creation proof.

1. derive and revoke transient Capability 26;
2. revoke Capability 13 through Agent Call 42;
3. compact Capabilities 14, 13, and 26 in child-first order;
4. archive Events 1 through 64 after Event 357 fills the live log;
5. retire Resource record 3 through Agent Call 41;
6. create fresh Service Resource 8 under root Resource 1, receiving Capability
   27 in a returned physical slot;
7. derive Capabilities 28 and 29 to refill the Capability Store;
8. submit and complete the Supervisor Task.

Required terminal evidence:

- Resource 3 and Capabilities 13, 14, and 26 no longer resolve;
- Resource 8 is active, owned by the Admission Supervisor, and occupies the
  seven-record Store without changing capacity;
- Capability 27 belongs to Resource 8, while Capabilities 28 and 29 prove the
  other sparse slots were reused;
- Resource and Capability IDs advance monotonically;
- Event 357 still triggers the full-log archive checkpoint;
- archived and live iterators form one exact sequence through Event 383;
- the Supervisor executes 38 Agent Calls and 76 address-space switches;
- debug and release QEMU agree on the complete transcript and archive digest.

The frozen Admission Supervisor artifact contains 3,765 code bytes and 3,797
Capsule bytes. Its SHA-256 digest is
`12d8f989d16454ce12d6de369033f00d70717ba8d7abee400168ca5610047b0b`.
The 38 return offsets are:

```text
44, 82, 169, 247, 361, 400, 495, 609, 648, 766, 889, 976, 1054,
1168, 1207, 1302, 1416, 1455, 1573, 1693, 1813, 1933, 2053, 2173,
2291, 2412, 2530, 2649, 2768, 2886, 3004, 3122, 3282, 3409, 3531,
3652, 3734, 3763
```

The independent assembly output and generated Rust Capsule are byte-exact.
The retained Event 1 through 64 archive digest remains
`bcca87f797c97d77eea510de12bd94142e469566817d2758b692287b0edabf67`.
Its four reply words remain, in little-endian order:

```text
777dc997f787cabc, 1494bd12de10a5ee,
58277d816695462e, 67bfda0e7b2892b6
```

The Supervisor tail is fixed at Events 352 through 364. Capability cleanup
revocation is Event 354, child-first Capability compaction occupies Events 355
through 357, Resource record retirement is Event 358, Resource 8 creation and
its initial Capability occupy Events 359 and 360, and refill derivations occupy
Events 361 and 362. Final verification, release, Image, and Driver work extends
the complete transcript through `DriverInvocationCompleted` at Event 383. The
final live Event occupancy is 319 and `next_event` is 384.

Each new validation marker occurs exactly once:

```text
AGENT_KERNEL_AGENT_CALL_CAPABILITY_CLEANUP_REVOCATION_OK
AGENT_KERNEL_AGENT_CALL_RESOURCE_RECORD_RETIREMENT_OK
AGENT_KERNEL_NATIVE_CAPABILITY_CLEANUP_REVOCATION_OK
AGENT_KERNEL_NATIVE_RESOURCE_RECORD_RETIREMENT_OK
AGENT_KERNEL_NATIVE_RESOURCE_STORE_REUSE_OK
```

## Failure Rules

- missing launched Supervisor: existing Agent Entry errors;
- non-Supervisor caller: `AgentEntryKindMismatch`;
- active Resource: `ResourceRecordRetirementNotReady`;
- Capability cleanup against an active Resource: `CapabilityCleanupNotReady`;
- retained Resource reference: `ResourceRecordRetirementReferenced`;
- revoked cleanup target: `CapabilityRevoked`;
- missing target: existing Resource or Capability lookup errors;
- invalid, revoked, foreign, task-scoped, attenuated, or unrelated authority:
  existing Capability and Resource scope errors;
- Event exhaustion: `EventLogFull` with all Stores unchanged.

## Validation

The final implementation passed:

- `cargo fmt --check`;
- `cargo test --workspace`;
- `cargo run -p agent-supervisor`;
- the bare-metal `x86_64-unknown-none` check;
- debug and release `scripts/run-qemu.sh` runs with the exact 383-Event
  transcript;
- independent assembly, Capsule SHA-256, all 38 symbol-derived return offsets,
  and one complete Capsule occurrence in the release ELF.

## Deferred Work

- batched subtree retirement in post-order;
- durable receipt storage and signed retirement checkpoints;
- compaction primitives for every Store that can retain a Resource;
- policy-controlled cascading cleanup for whole Resource domains;
- architecture-neutral external Resource archives.
