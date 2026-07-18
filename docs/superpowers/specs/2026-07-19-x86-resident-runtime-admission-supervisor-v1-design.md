# X86 Resident Runtime Admission Supervisor V1 Design

## Status

Implemented and validated on 2026-07-19.

## Purpose

Runtime Admission Protocol V1 proves that a ring-3 Supervisor can create two
audited requests and that the x86 broker can consume them. The Supervisor then
completes before either admitted Worker starts. This leaves the control plane
outside the execution interval it authorized.

Resident Runtime Admission Supervisor V1 keeps that ring-3 control plane alive
across physical admission, concurrent Worker ownership, target execution, and
completion notification. It composes the existing Runtime Admission object,
blocking Mailbox IPC, fixed-capacity scheduler, native runtime registry, and
address-space reclamation contracts without adding a privileged shortcut.

## Resident Lifecycle

The reference flow has seven bounded phases:

1. Agent 10 and Agent 11 are registered, launched, and accepted without queue
   visibility.
2. Agent 12 is launched as the Admission Supervisor and receives one private
   eleven-frame address space through the fixed bootstrap boundary.
3. The Supervisor executes `DescribeContext`, submits two
   `RequestRuntimeAdmission` calls, and executes `ReceiveMessage` with an empty
   Mailbox. The Task enters `Waiting`, and the retained call frame remains in
   the native runtime.
4. The broker admits Agent 10 and Agent 11 in request order while Agent 12 stays
   resident. All three private address spaces are live together.
5. Each Worker executes, submits its result, sends a `Notify` message carrying
   its own Task ID to Agent 12, and completes. The first message wakes and queues
   Agent 12. The second message remains pending in FIFO order.
6. Agent 12 is dispatched from its retained receive frame, receives and
   acknowledges both notifications, submits its result, and completes.
7. The bootstrap verifier verifies both Workers and the Supervisor. One bounded
   three-owner reclamation returns all 33 private frames to the zeroed pool.

## Notification Contract

The Worker Capsule uses the existing authenticated Agent Call 7. Its trusted
execution identity supplies sender Agent, sender Task, Image, and nonce. The
request carries:

- recipient Agent 12;
- `MessageKind::Notify`;
- the sender's trusted Task ID as the bounded payload;
- zero in every unsupported payload register.

The Supervisor validates the FIFO replies before acknowledgement:

| Notice | Sender | Task payload |
| --- | ---: | ---: |
| First | Agent 10 | Task 8 |
| Second | Agent 11 | Task 9 |

Both records must reach `Acknowledged`. The resident Mailbox waiter must become
inactive after the first wake. Any sender, recipient, kind, Task, ordering,
status, or transcript mismatch terminates the reference boot.

## Agent Call Transcripts

The Supervisor owns this nine-call transcript:

1. `DescribeContext`;
2. `RequestRuntimeAdmission` for Agent 10 / Task 8;
3. `RequestRuntimeAdmission` for Agent 11 / Task 9;
4. blocking `ReceiveMessage`;
5. `AcknowledgeMessage` for the first notification;
6. immediate `ReceiveMessage` for the queued second notification;
7. `AcknowledgeMessage` for the second notification;
8. `SubmitTaskResult`;
9. `CompleteTask`.

Each Worker owns `DescribeContext`, `SubmitTaskResult`, `SendMessage`, and
`CompleteTask`. Return offsets come from assembly symbols and become immutable
Capsule evidence together with exact SHA-256 digests. The Supervisor performs
18 Agent/kernel address-space switches; the two Workers perform 16 combined
switches.

## Capsule Evidence

Both assembly sources were rebuilt independently and compared byte-for-byte
with their Rust `static` Capsule authorities and the release kernel ELF. Each
complete Capsule occurs exactly once in the final ELF.

| Capsule | Capsule bytes | Code bytes | Return offsets | Capsule SHA-256 | Code SHA-256 | Release ELF instances |
| --- | ---: | ---: | --- | --- | --- | ---: |
| Runtime Service Worker | 165 | 133 | 46, 67, 102, 131 | `6dcc2d51c7b08dc21593f94d064b1ca809a2b4d7e0e2fbbd7ac44992086eba1e` | `01ae3cfadec6b264a886e80a910ecf664e1d9d4c72e03bef49a68b2c00ef16a7` | 1 |
| Admission Supervisor | 600 | 568 | 44, 82, 169, 247, 358, 395, 486, 537, 566 | `23f31f028d2356bd75cd57fc81ff50e6979d96d67b0422da4b63dbedadea7c2a` | `41975c95455e746ddf8a72fd5439979eb405bd0cafaa235879bfe5d891d2b7c6` | 1 |

## Physical Ownership

The private frame pool starts with 66 zeroed frames after the first six native
address spaces are reclaimed.

| Boundary | Pool frames | Native runtime contexts |
| --- | ---: | ---: |
| Supervisor waiting | 55 | 1 |
| First Worker admitted | 44 | 2 |
| Rejected duplicate registration rolled back | 44 | 2 |
| Second Worker admitted | 33 | 3 |
| All three completed | 33 | 0 |
| Three-owner terminal reclamation | 66 | 0 |

The duplicate registration proof remains in the flow. Its cancelled identity
must be reused by the second Worker, and every live address space must remain
pairwise disjoint.

## Exact Event Tail

Events 1 through 227 remain unchanged. The resident tail is:

| Events | Transition |
| --- | --- |
| 228-230 | Supervisor dispatch, quantum expiry, redispatch |
| 231-232 | two `RuntimeAdmissionRequested` records |
| 233 | Supervisor `MessageWaitStarted` |
| 234-237 | two admission commits and two atomic Task queue entries |
| 238-242 | Worker FIFO dispatch and quantum expiries |
| 243-246 | Worker 10 result, notification, Supervisor wake, completion |
| 247-250 | Worker 11 dispatch, result, notification, completion |
| 251-257 | Supervisor dispatch, two receive/acknowledge pairs, result, completion |
| 258-263 | Worker 10, Worker 11, and Supervisor verification/fulfillment |
| 264-273 | UART-backed Driver invocation flow |

The final line is `event[273] driver_invocation_completed` followed by
`SUPERVISOR_HANDOFF_READY`.

## Deterministic Reference Profile

| Evidence | Count |
| --- | ---: |
| Registered Agents | 12 |
| Native ring-3 completions | 9 |
| Kernel-selected dispatches | 30 |
| Admission Supervisor Agent Calls | 9 |
| Admission Supervisor address-space switches | 18 |
| Runtime Service Worker Agent Calls | 8 |
| Runtime Service Worker address-space switches | 16 |
| Physical quantum expiries | 13 |
| Runtime admission requests | 2 |
| Runtime admission commits | 2 |
| Worker completion notifications | 2 |
| Resident Supervisor Mailbox waits | 1 |
| Resident Supervisor Mailbox wakes | 1 |
| Address-space cancellation cycles | 1 |
| Frames restored by admission cancellation | 11 |
| Terminal address-space reclamations | 9 |
| Terminal private-frame returns | 99 |
| Final zeroed private frame pool | 66 |
| Ordered kernel Events | 273 |

## QEMU Evidence

The reference boot adds these exact-once proofs:

```text
AGENT_KERNEL_NATIVE_RUNTIME_ADMISSION_RESIDENT_WAIT_OK
AGENT_KERNEL_NATIVE_RUNTIME_ADMISSION_NOTIFICATION_OK
```

Existing Runtime Admission, cancellation, concurrent ownership, execution,
reclamation, Agent Call, and Driver markers remain required. The strict script
requires the exact Mailbox event sequence, both new exact-once markers, and 273
Events in debug and release profiles. Completed CPU transcripts prove the nine
Supervisor calls and four calls from each Worker.

## Validation

- The strict QEMU contract failed on the missing resident-wait marker before the
  Capsule and flow changes.
- Independent assembly, Rust authority, and release ELF audits froze the exact
  Capsule bytes, hashes, operation order, return offsets, and one-instance
  linkage contract.
- Strict debug and release QEMU boots passed the exact marker counts, Mailbox
  event sequence, 273-event log, and terminal handoff.
- The full workspace test suite and host Supervisor flow passed.
- All five `no_std` library target checks and the freestanding x86 binary check
  passed for `x86_64-unknown-none`.
- Workspace and freestanding Clippy passed with warnings denied, the forbidden
  host/allocation API scan was empty, and `cargo fmt --check` passed.
- Both README languages describe the resident lifecycle and deterministic
  reference evidence.

## Deferred Work

- a terminal `RuntimeAdmissionReleased` semantic transition after physical
  address-space reclamation;
- discovery of the requesting Supervisor identity by target Capsules;
- cancellation and fault notifications for pending or active targets;
- a repeated resident control loop spanning multiple admission batches;
- dynamic private page-table growth, SMP synchronization, PCID lifecycle, and
  hardware TLB shootdown.
