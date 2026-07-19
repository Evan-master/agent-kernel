# Message Retirement V1 Design

## Status

Implemented and validated on 2026-07-19.

## Purpose

Agent Mailbox IPC stores every sent message in a fixed-capacity dense array.
Receive and acknowledge close the delivery protocol, while the acknowledged
record continues to occupy its slot indefinitely. A resident Agent that handles
repeated notifications will therefore exhaust the Message Store even though
every earlier delivery has reached its terminal state.

Message Retirement V1 gives the recipient explicit control over terminal
mailbox retention. It removes one acknowledged record, preserves ordered Event
evidence, keeps Message IDs monotonic, and returns the vacated slot to the fixed
store. The operation remains an Agent-identity lifecycle transition; message
payload references do not grant or transfer authority.

## Identity And Authority

`retire_message(agent, message)` is owned by the message recipient.

- the caller must be an active registered Agent;
- the Message must exist;
- the caller must exactly match `MessageRecord::recipient`;
- suspended, retired, unknown, or foreign Agents fail before mutation;
- no Resource Capability is accepted because Mailbox IPC belongs to the Agent
  identity domain established by Send, Receive, and Acknowledge.

The architecture adapter authenticates the caller through the scheduler-owned
Agent, Task, Image, nonce, running Task, admitted launch Entry, and private CR3.
The caller cannot select another Agent identity through the retirement payload.

## Terminal And Reference Rules

Only `MessageStatus::Acknowledged` records may retire. `Pending` and `Received`
records continue to represent live delivery state and return
`MessageStatusMismatch`.

A live Namespace entry containing `NamespaceObject::Message(message)` blocks
retirement with `MessageRetirementReferenced`. Events may retain Message IDs as
immutable audit evidence. Inactive Waiters and acknowledged payload references
also remain historical evidence and do not keep a Message record live.

Agent identity retirement and cleanup policy for abandoned pending mail remain
separate lifecycle work.

## Dense Store Mutation

Retirement performs a complete read-only preflight:

1. validate the active caller;
2. locate and copy the Message record;
3. validate exact recipient ownership;
4. validate acknowledged terminal state;
5. reject a live Namespace reference;
6. reserve one Event slot.

After preflight, later records shift left by one position, the vacated tail is
reset to `MessageRecord::empty()`, and `message_len` decreases by one. Relative
order among retained records remains unchanged. `next_message` never decreases,
so a later Send reuses physical capacity with a fresh Message ID.

Every failed preflight preserves Message contents, order, count, next ID, and
Event length.

## Receipt And Event

`MessageRetirement` is a copyable receipt containing the complete retired
`MessageRecord`. It exposes the retired Message ID and immutable record without
retaining a store pointer.

One `MessageRetired` Event records:

- the recipient and retiring Agent in `agent`;
- the sender in `target_agent`;
- the retired Message ID in `message`;
- the Message kind in `message_kind`;
- every optional Resource, Capability, Intent, Task, Action, and Fault payload
  reference in its matching Event field;
- the next global Event sequence.

The earlier Send, Receive, and Acknowledge Events preserve the complete ordered
delivery lifecycle.

## Facade Contract

`AgentKernel::sys_retire_message(agent, message)` exposes the same operation and
returns `MessageRetirement`. Read-only `messages()` inspection immediately
reflects dense removal.

Core and facade tests must prove:

- acknowledged retirement and exact receipt/Event fields;
- middle-record removal with retained FIFO order;
- physical slot reuse with a strictly increasing Message ID;
- rejection of pending, received, foreign, inactive, unknown, and missing
  Messages without mutation;
- Namespace reference rejection;
- Event-capacity failure atomicity.

## Agent Call 34

The native ABI adds:

```text
RetireMessage = 34
```

Request payload:

| Register | Value |
| --- | --- |
| `r10` | acknowledged Message ID |
| `r11-r15`, `rbp` | zero |

The authenticated Agent, Task, Image, and nonce remain in `rsi`, `rdi`, `r8`,
and `r9`.

Successful reply payload:

| Register | Value |
| --- | --- |
| `r10` | retired Message ID |
| `r11-r15`, `rbp` | zero |

Malformed, unauthenticated, foreign-recipient, nonterminal, referenced, missing,
and Event-capacity failures fail closed.

## X86 Proof

The resident Runtime Admission Supervisor receives four completion notices
across two execution batches. After each Acknowledge call, it invokes Agent Call
34 for the same Message ID and validates the canonical reply before continuing.

The two earlier acknowledged boot-flow Messages remain stored. Admission
Messages 3 through 6 reuse two trailing physical Message slots across the two
batches. The x86 Message capacity therefore falls from six records to four
while preserving the same six monotonic Message IDs across the full boot.

The final store contains only Messages 1 and 2. Four ordered `MessageRetired`
Events prove recipient ownership and retirement. The strict QEMU contract also
freezes the expanded Capsule bytes, SHA-256 digest, return offsets, Agent Call
count, address-space switch count, exact Event sequence, final Event count, and
single release-ELF Capsule occurrence.

The independently assembled artifact freezes these values:

- machine code: 2762 bytes;
- complete Capsule: 2794 bytes;
- SHA-256: `5ca8ce958b829fa3f7663219ca36f56da95e3918027a21d9ed395e3cd6e4ec25`;
- return offsets: `44, 82, 169, 247, 361, 400, 495, 609, 648, 769, 856,
  934, 1048, 1087, 1182, 1296, 1335, 1453, 1573, 1693, 1813, 1931,
  2052, 2170, 2289, 2407, 2528, 2649, 2731, 2760`;
- Supervisor transcript: 30 Agent Calls and 60 Agent/kernel address-space
  switches;
- Message Store capacity: four records;
- terminal Message records: IDs 1 and 2;
- retirement Events: sequences 284, 287, 317, and 320;
- final Event count: 362.

The complete Capsule occurs exactly once in the validated release ELF.

## Failure Rules

- Unknown caller returns `AgentNotFound`.
- Suspended or retired caller returns its existing Agent lifecycle error.
- Missing Message returns `MessageNotFound`.
- Foreign recipient returns `MessageAgentMismatch`.
- Pending or Received state returns `MessageStatusMismatch`.
- Live Namespace binding returns `MessageRetirementReferenced`.
- Event exhaustion returns `EventLogFull` before dense-store mutation.

## Validation

- Eight Core tests cover terminal retirement, receipt and Event fields, dense
  middle removal, FIFO retention, monotonic IDs, capacity reuse, ownership,
  lifecycle state, Namespace liveness, missing records, and Event atomicity.
- One facade test proves the public syscall lifecycle and one-slot reuse.
- Three x86 ABI tests freeze Call 34 decoding, reserved-register rejection,
  authentication, and canonical reply encoding.
- Full workspace tests and the host Supervisor run pass.
- The freestanding `x86_64-unknown-none` bare-metal target passes `cargo check`.
- Full-target Clippy passes with warnings denied and the established
  const-generic `too_many_arguments` allowance.
- Debug and release strict QEMU runs each prove all 362 ordered Events and exact
  marker multiplicities.
- Independent assembly, frozen-array comparison, SHA-256 verification, and
  release-ELF scanning prove one exact Capsule instance.

## Deferred Work

- Agent identity compaction and execution-context slot reuse;
- policy and administrative cleanup for pending mail owned by a retired Agent;
- Namespace unbind and Namespace entry retirement;
- bounded Event archival and durable replay checkpoints;
- multicast, timeout, rejection, and delivery-dead-letter policy.
