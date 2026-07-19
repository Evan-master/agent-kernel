# X86 Resident Runtime Admission Batches V1 Design

## Status

Implemented and validated on 2026-07-19.

## Purpose

The current Runtime Admission reference path keeps one ring-3 Supervisor
resident while two Workers execute, then completes and reclaims all three
address spaces. It proves one admission batch, one notification round, and one
terminal semantic release.

This milestone extends that lifecycle to two sequential batches under the same
Supervisor CPU context. Batch one must be verified, physically reclaimed, and
semantically released while the Supervisor remains blocked on its second
Mailbox wait. Batch two then consumes the newly returned zeroed frames and
completes through the same authenticated protocol.

## Fixed Identities

Preparation order preserves the existing Supervisor ABI identities:

| Role | Agent | Task | Image | Task Capability |
| --- | ---: | ---: | ---: | ---: |
| Batch-one Worker A | 10 | 8 | 9 | 20 |
| Batch-one Worker B | 11 | 9 | 10 | 21 |
| Resident Supervisor | 12 | 10 | 11 | 22 |
| Batch-two Worker A | 13 | 11 | 12 | 24 |
| Batch-two Worker B | 14 | 12 | 13 | 25 |

The Supervisor retains root-scoped admission authority Capability 23. Batch
two is prepared after the Supervisor so its Task, Image, and Capability values
remain stable.

## Supervisor Transcript

The Supervisor executes one continuous fifteen-call transcript:

```text
DescribeContext
RequestRuntimeAdmission   # Agent 10, Task 8
RequestRuntimeAdmission   # Agent 11, Task 9
ReceiveMessage
AcknowledgeMessage
ReceiveMessage
AcknowledgeMessage
RequestRuntimeAdmission   # Agent 13, Task 11
RequestRuntimeAdmission   # Agent 14, Task 12
ReceiveMessage
AcknowledgeMessage
ReceiveMessage
AcknowledgeMessage
SubmitTaskResult
CompleteTask
```

Each request reply validates its kernel-issued Admission ID, target Agent, and
target Task. Each notification validates sender, `Notify` kind, Task payload,
and zero reserved fields. Both empty receives transfer the same owned CPU frame
into a kernel Mailbox waiter.

## Batch-One Boundary

After the first two Workers complete and the Supervisor requests batch two:

- native runtime contains only the waiting Supervisor;
- the completion report owns exactly the two batch-one Worker CPUs;
- Runtime Admissions 1 and 2 are `Admitted`;
- Runtime Admissions 3 and 4 are `Requested`;
- Waiter 4 owns the active second Supervisor Mailbox wait;
- the address-space frame pool contains 33 zeroed frames;
- both batch-one notifications are `Acknowledged`;
- no batch-two target is queued or physically admitted.

The bootstrap verifier then verifies Tasks 8 and 9. A two-record semantic
release permit is prepared before physical ownership moves.

## Partial Physical Reclamation

`NativeExecutionReport::reclaim_completed_address_spaces` becomes valid for a
complete subset of currently completed CPUs. It still requires:

- a nonempty unique Agent list;
- exact equality between report length and requested reclaim count;
- no faulted CPU owner;
- copied-ledger preflight for every complete eleven-frame identity;
- deterministic commit order;
- an empty completion report after transfer;
- every returned identity present and physically zero in the pool.

Full-pool occupancy is a caller-level terminal condition. Batch one reclaims 22
frames and raises the pool from 33 to 55 while the Supervisor keeps its own 11
frames. Runtime Admissions 1 and 2 become `Released` only after this physical
transfer succeeds. Admissions 3 and 4 remain `Requested`.

## Cross-Batch Frame Reuse

The frame pool allocates complete address spaces from its tail. With ordered
batch-one reclamation, batch-two admission proves:

- Agent 13 receives the former Agent 11 address-space identity;
- Agent 14 receives the former Agent 10 address-space identity;
- both identities remain disjoint from the resident Supervisor;
- every consumed frame was zero before page-table reconstruction;
- pool occupancy returns from 55 to 33.

This is physical reuse across a semantic release boundary, with no address
space shared by simultaneous owners.

## Terminal Boundary

After batch two:

1. Workers 13 and 14 notify and complete;
2. the Supervisor acknowledges both notifications, submits its result, and
   completes;
3. Tasks 11, 12, and 10 are verified;
4. a release permit is prepared for Admissions 3 and 4;
5. the final three address spaces return 33 zeroed frames;
6. the pool reaches all 66 zeroed frames;
7. Admissions 3 and 4 commit `Released` in FIFO order.

All four Runtime Admission records remain queryable for audit.

## Capacity Profile

The deterministic boot profile grows only where the second batch needs owned
records:

| Store | Current | Two-batch profile |
| --- | ---: | ---: |
| Agents and Agent Images | 12 | 14 |
| Capabilities | 23 | 25 |
| Intents | 10 | 12 |
| Tasks and Runtime Admissions | 10 | 12 |
| Messages | 4 | 6 |
| Waiters | 3 | 4 |
| Events | 275 | 326 |

Resource, run-queue, native-runtime, memory, fault, and Driver capacities stay
unchanged.

## Deterministic Evidence

The expected reference totals are:

| Evidence | Count |
| --- | ---: |
| Registered Agents | 14 |
| Native ring-3 completions | 11 |
| Kernel-selected dispatches | 35 |
| Physical quantum expiries | 15 |
| Supervisor Agent Calls | 15 |
| Supervisor address-space switches | 30 |
| Runtime Service Worker Agent Calls | 20 |
| Runtime Service Worker address-space switches | 40 |
| Runtime Admission requests | 4 |
| Runtime Admission commits | 4 |
| Runtime Admission requester discoveries | 4 |
| Runtime Admission releases | 4 |
| Worker completion notifications | 4 |
| Supervisor Mailbox waits and wakes | 2 each |
| Native address-space reclamation completions | 11 |
| Cumulative private-frame returns | 121 |
| Final zeroed frame pool | 66 |
| Ordered kernel Events | 326 |

The new phase markers are:

```text
AGENT_KERNEL_NATIVE_ADDRESS_SPACE_PARTIAL_RECLAIM_OK
AGENT_KERNEL_NATIVE_RUNTIME_ADMISSION_REPEAT_OK
```

Existing per-batch Runtime Admission markers occur twice. Request and discovery
Agent Call markers occur four times.

## Failure And Atomicity

- A malformed second request or notification enters the Supervisor halt loop.
- Batch-one verification failure stops before release preparation.
- Partial reclamation preflight failure retains every completed CPU owner.
- Any failed physical transfer stops before semantic release.
- A stale release permit changes no Runtime Admission record or Event.
- Batch-two admission failure preserves the existing rollback behavior.
- Frame identity mismatch, nonzero returned bytes, retained-owner overlap, or
  FIFO mismatch fails the boot before the next phase.
- Read-only Agent Calls continue to emit no semantic Event.

## Validation

- Add a host contract for reclaim, allocate, partial return, and cross-batch
  identity reuse in a three-address-space ledger.
- Make strict QEMU require both new phase markers before implementation.
- Rebuild the Supervisor Capsule and freeze its exact bytes, hashes, operations,
  and return offsets.
- Validate both resident waits, both release boundaries, all four records, and
  cross-batch physical identities.
- Run formatting, full workspace tests, host Supervisor, `no_std` and
  freestanding checks, warning-free Clippy, strict debug/release QEMU, and
  release ELF Capsule audits.
- Update both README languages and publish public `main`.

## Validation Result

- Supervisor Capsule: 1,090 bytes; executable code: 1,058 bytes.
- Capsule SHA-256:
  `e663c30c5d0c110c50fc2d425d156772fb739f842c59f969f086b92858303dd6`.
- Code SHA-256:
  `074826564da8e3ae09354e2a11f434f7d7aaeb5d5c95773f86cf2f78e10dd0b9`.
- Return offsets:
  `[44, 82, 169, 247, 358, 395, 506, 572, 659, 737, 848, 885, 976, 1027, 1056]`.
- Strict debug and release QEMU each completed all 326 ordered Events and
  reached `SUPERVISOR_HANDOFF_READY`.
- The full workspace tests, host Supervisor run, `no_std` checks, freestanding
  x86_64 check, formatting, and shell validation passed.
- Workspace and freestanding Clippy passed with warnings denied after retaining
  the repository's structural allowance for eight pre-existing
  `too_many_arguments` sites.
- The release ELF contains one exact Capsule occurrence and one exact code
  occurrence; the executable bytes match the independently assembled `.text`
  section byte for byte.

## Deferred Work

- an unbounded userspace-created batch loop;
- bounded terminal Runtime Admission record compaction;
- a Runtime Admission queue larger than the Task store;
- cancellation and fault disposition notifications;
- dynamic page-table growth, SMP synchronization, PCID lifecycle, and hardware
  TLB shootdown.
