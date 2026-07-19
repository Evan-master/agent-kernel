# Orphaned Message Retirement V1 Design

## Status

Implemented, validated, and published on 2026-07-19.

## Purpose

Mailbox delivery validates that a recipient is active when a Message enters
the fixed-capacity store. A managed Agent may retire while earlier Messages
remain `Pending`. The retired recipient cannot receive, acknowledge, or invoke
recipient-owned Message retirement, so each abandoned record permanently
occupies one Message slot.

Orphaned Message Retirement V1 gives an authorized Agent administrator a
bounded cleanup primitive. It removes one pending Message whose managed
recipient has retired, preserves ordered audit evidence, and returns the slot
to the fixed store without reusing a Message ID.

## Identity And Authority

`retire_orphaned_message(actor, authority, message)` requires:

1. an active registered actor;
2. an existing Message;
3. a `Pending` Message status;
4. a registered recipient in `AgentStatus::Retired`;
5. a managed recipient with both `manager` and `management_resource` recorded;
6. an active, root-scoped Capability held by the actor for that exact
   management Resource;
7. `Operation::Delegate` in the Capability operation set.

The recorded original manager does not hold implicit ambient authority. A
delegated administrator may perform cleanup only while its complete Capability
ancestry remains active. Trusted bootstrap Agents without a management Resource
have no administrative Mailbox cleanup path through this operation.

The actor cannot select a recipient separately. The kernel derives the target
from the immutable Message record and derives the authorization scope from the
retired Agent record.

## Eligibility And References

Only `MessageStatus::Pending` is eligible. `Received` and `Acknowledged`
Messages return `MessageStatusMismatch`; recipient-owned retirement remains the
terminal path for acknowledged delivery.

An active or suspended recipient returns
`OrphanedMessageRetirementNotReady`. A retired unmanaged recipient returns
`AgentManagementDenied`.

A live Namespace entry containing `NamespaceObject::Message(message)` blocks
removal with `MessageRetirementReferenced`. Earlier Events remain immutable
historical references and do not keep the record live.

## Atomic Dense Removal

The operation performs a read-only preflight before mutation:

1. validate the actor lifecycle;
2. locate and copy the Message;
3. validate Pending state;
4. validate retired managed recipient state;
5. authorize exact Delegate authority;
6. reject a Namespace reference;
7. reserve one Event slot.

After preflight, the shared dense Message removal helper shifts later records
left, clears the old tail with `MessageRecord::empty()`, and decrements
`message_len`. Relative FIFO order remains stable. `next_message` remains
monotonic, so subsequent sends reuse physical capacity with fresh identities.

Every failure preserves Message contents, order, count, next ID, and Event
length.

## Receipt And Event

`OrphanedMessageRetirement` contains:

- the complete removed `MessageRecord`;
- the administrative actor;
- the authorizing Capability;
- the recipient's management Resource.

One `OrphanedMessageRetired` Event records:

- the administrative actor in `agent`;
- the retired recipient in `target_agent`;
- the Message ID and kind;
- the authorizing Capability in `source_capability`;
- `Operation::Delegate`;
- every optional Resource, Capability, Intent, Task, Action, and Fault payload
  reference in its matching Event field.

The earlier `MessageSent` Event preserves the sender and recipient. Together,
the ordered Events provide complete delivery and administrative disposition
evidence.

## Facade Contract

`AgentKernel::sys_retire_orphaned_message(actor, authority, message)` exposes
the Core operation unchanged and returns `OrphanedMessageRetirement`.

Core and Facade tests prove authorization, delegated authority, lifecycle
eligibility, Pending-only policy, Namespace liveness, dense-order preservation,
monotonic IDs, capacity reuse, complete evidence, and Event-full atomicity.

## Agent Call 35

The native ABI adds:

```text
RetireOrphanedMessage = 35
```

Request payload:

| Register | Value |
| --- | --- |
| `r10` | Delegate authority Capability ID |
| `r11` | orphaned Message ID |
| `r12-r15`, `rbp` | zero |

The scheduler-authenticated Agent, Task, Image, and nonce remain in `rsi`,
`rdi`, `r8`, and `r9`.

Successful reply payload:

| Register | Value |
| --- | --- |
| `r10` | retired Message ID |
| `r11` | retired recipient Agent ID |
| `r12` | management Resource ID |
| `r13-r15`, `rbp` | zero |

The architecture executor validates the receipt, exact Event evidence, Message
absence, and unchanged running context before issuing the canonical reply.

## X86 Proof

The ring-3 Resource Manager registers managed Agent 9, sends one Pending
Message to it, retires the quiescent identity through Agent Call 20, and then
retires the orphan through Agent Call 35 using Capability 12. The Capsule checks
the Message ID, retired recipient, and management Resource in the reply.

The ordered proof records Agent 9 registration at Event 170, Message 3 delivery
at Event 171, suspend/resume/retire at Events 172 through 174, and orphaned
Message retirement at Event 175. Runtime Admission completion notices then use
Messages 4 through 7 while the four-slot Message Store retains only historical
boot Messages 1 and 2.

The Resource Manager proof is frozen at:

- 31 authenticated Agent Calls and 62 Agent/kernel address-space switches;
- 2,800 machine-code bytes and a 2,832-byte Capsule;
- SHA-256
  `724f0da14c9a69b9e89ac2c7ea9f70559803a40e1cb1762800218ab9862256ec`;
- return offsets `45, 86, 163, 236, 310, 390, 463, 539, 626, 710, 828, 912,
  996, 1080, 1206, 1280, 1399, 1492, 1582, 1659, 1805, 1933, 2010, 2156,
  2250, 2378, 2455, 2601, 2715, 2789, 2798`.

The Message-ID update in the Admission Supervisor remains frozen at 2,762
machine-code bytes and a 2,794-byte Capsule with SHA-256
`aea6f5f466ea2fffbbb5c39b7c570e20824563b3e8bfdfe94adbd858c51f9011`.
Its 30 return offsets remain `44, 82, 169, 247, 361, 400, 495, 609, 648, 769,
856, 934, 1048, 1087, 1182, 1296, 1335, 1453, 1573, 1693, 1813, 1931, 2052,
2170, 2289, 2407, 2528, 2649, 2731, 2760`.

Strict Debug and release QEMU runs require 364 ordered Events, exactly one
per-call marker, exactly one aggregate orphan-retirement marker, and Event 364
as `driver_invocation_completed`. Independently assembled bytes match the
generated Rust arrays, and each complete Capsule occurs exactly once in the
release ELF.

## Validation

- nine Core contract tests cover successful evidence, delegated authority,
  recipient lifecycle, Message status, unmanaged recipients, caller and
  Capability authorization, Namespace liveness, dense middle removal,
  monotonic reuse, and failure atomicity;
- one Facade contract proves public syscall routing and fixed-slot reuse;
- three x86 ABI contracts prove decode/authentication, malformed-frame
  rejection, reserved-register rejection, and canonical reply encoding;
- strict Debug and release QEMU executions complete with the exact 364-Event
  serial transcript and success exit status;
- independent assembly, SHA-256, generated-byte equality, return-offset, and
  release-ELF occurrence audits pass for both persistent Supervisor Capsules.

## Failure Rules

- unknown actor: `AgentNotFound`;
- suspended or retired actor: existing Agent lifecycle error;
- missing Message: `MessageNotFound`;
- Received or Acknowledged Message: `MessageStatusMismatch`;
- active or suspended recipient: `OrphanedMessageRetirementNotReady`;
- retired unmanaged recipient: `AgentManagementDenied`;
- missing, foreign, task-scoped, revoked, attenuated, or wrong-operation
  authority: existing Capability authorization error;
- live Namespace binding: `MessageRetirementReferenced`;
- full Event Log: `EventLogFull` before Message mutation.

## Deferred Work

- sender cancellation before delivery;
- dead-letter routing and retry policy;
- multicast and delivery timeout policy;
- Agent registry and execution-context slot reuse;
- Namespace unbind and Namespace entry retirement;
- bounded Event archival and durable replay checkpoints.
