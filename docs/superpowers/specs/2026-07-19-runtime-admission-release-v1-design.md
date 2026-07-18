# Runtime Admission Release V1 Design

## Status

Implemented and validated on 2026-07-19.

## Purpose

Runtime Admission currently records `Requested`, `Admitted`, and `Rejected`.
The x86 runtime can reclaim every private frame after a verified Agent finishes,
while the corresponding semantic record remains `Admitted`. The event log
therefore cannot prove that physical ownership ended.

Release V1 closes that lifecycle. A fixed-capacity, generation-bound batch
permit preflights every semantic release. The x86 owner then reclaims the full
physical batch and commits `RuntimeAdmissionReleased` only after the frame pool
has accepted and zeroed every returned frame.

## State Machine

The valid transitions are:

```text
Requested -> Admitted -> Released
Requested -> Rejected
```

`Released` and `Rejected` are terminal in V1. A released record preserves its
requester, authority, target, Task, Image, Resource, and stable admission ID.
Its failure remains empty.

## Batch Release Permit

`RuntimeAdmissionReleaseBatch<const COUNT: usize>` contains:

- the exact ordered admission record snapshots;
- the global Runtime Admission generation observed during preparation.

Preparation is read-only. It succeeds only when:

- `COUNT` is greater than zero and fits the Runtime Admission store;
- every requested admission ID is nonzero, present, and unique in the batch;
- every record is `Admitted` with no failure;
- every target Task is `Verified`, still assigned to the recorded target Agent,
  and still bound to the recorded Resource;
- every target execution context is `Idle` with no Task or Driver invocation;
- the event log has `COUNT` free slots.

The batch exposes immutable record inspection and keeps its generation private
to the core transition implementation.

## Atomic Commit

Commit revalidates the complete batch before the first mutation:

1. the global generation still matches the permit;
2. all record snapshots still match core storage;
3. all release-readiness conditions still hold;
4. all target records are distinct;
5. the aggregate event capacity remains available.

After preflight, every record changes to `Released` in permit order, one
`RuntimeAdmissionReleased` event is appended for each record, and the global
generation advances once for the complete batch. Any preflight error leaves
records, events, and generation unchanged.

## Physical Ordering

The x86 resident admission flow uses this order:

1. verify both Runtime Service Worker Tasks and the resident Supervisor Task;
2. prepare one two-record release batch for the admitted Workers;
3. preflight and reclaim the Supervisor and both Worker address spaces as one
   three-owner physical transaction;
4. prove all 33 frames are zeroed and restored to the 66-frame pool;
5. prove the semantic event count has not changed during physical reclamation;
6. commit the two-record semantic release batch;
7. validate both released records and the exact ordered release events.

No kernel semantic mutation occurs between permit preparation and commit. The
physical frame pool and native completion report live in the architecture
layer, so their transaction does not invalidate the semantic generation.

## Exact Event Tail

Events 1 through 263 retain the Resident Runtime Admission Supervisor V1
contract. The new terminal tail is:

| Events | Transition |
| --- | --- |
| 264-265 | Runtime Admission 1 and 2 released in FIFO request order |
| 266-275 | UART-backed Driver invocation flow |

The final line is `event[275] driver_invocation_completed`, followed by
`SUPERVISOR_HANDOFF_READY`.

## Deterministic Evidence

The reference boot adds:

```text
AGENT_KERNEL_NATIVE_RUNTIME_ADMISSION_RELEASE_OK
```

The marker is emitted exactly once after both semantic records and both release
events are validated. Existing physical reclamation, resident wait,
notification, execution, and final handoff markers remain mandatory.

The reference profile changes these counts:

| Evidence | Count |
| --- | ---: |
| Runtime Admission releases | 2 |
| Terminal released records | 2 |
| Ordered kernel Events | 275 |
| Final zeroed private frame pool | 66 |

All Capsule bytes, hashes, operation transcripts, dispatch counts, physical
quantum counts, and address-space switch counts remain unchanged.

## Failure And Atomicity

- Empty and duplicate batches fail before mutation.
- Missing, pending, rejected, released, or unverified admissions fail before
  mutation.
- Active target execution contexts fail release preparation.
- Event-capacity failure leaves every record admitted.
- Any semantic mutation after preparation makes the batch stale.
- A stale commit emits no event and changes no record.
- The x86 reference path stops before semantic release if physical reclamation
  or zero verification fails.

## Validation

- The first focused core run failed on the missing release state, permit,
  errors, events, and transition methods.
- Core tests cover release readiness, read-only preparation, ordered two-record
  commit, empty and duplicate batches, aggregate event capacity, stale permits,
  and distinct Task owner and Admission requester identities.
- The facade test proves preparation and commit cross the public kernel boundary
  through the opaque batch value.
- The strict QEMU contract first failed on the missing release marker.
- An over-constrained owner/requester equality check triggered a fail-closed
  integration boot, was removed, and gained a dedicated regression test.
- Strict debug and release QEMU boots passed the exact release marker, two
  release events, 275-event sequence, and terminal handoff.
- The full workspace test suite and host Supervisor flow passed.
- All five `no_std` library checks and the freestanding x86 binary check passed
  for `x86_64-unknown-none`.
- Workspace and freestanding Clippy passed with warnings denied, the forbidden
  host/allocation API scan was empty, and formatting passed.

## Artifact Audit

The release ELF preserves the resident milestone's immutable Capsule bytes.
Each Rust `static` authority matches its independently assembled Capsule and
occurs exactly once in the final ELF.

| Capsule | Bytes | Release ELF instances | SHA-256 |
| --- | ---: | ---: | --- |
| Runtime Service Worker | 165 | 1 | `6dcc2d51c7b08dc21593f94d064b1ca809a2b4d7e0e2fbbd7ac44992086eba1e` |
| Admission Supervisor | 600 | 1 | `23f31f028d2356bd75cd57fc81ff50e6979d96d67b0422da4b63dbedadea7c2a` |

## Deferred Work

- discovery of the requesting Supervisor identity by admitted Capsules;
- repeated resident admission batches and bounded release-record compaction;
- cancellation and fault release dispositions;
- release notification delivery to the requesting Supervisor;
- dynamic page-table growth, SMP synchronization, PCID lifecycle, and hardware
  TLB shootdown.
