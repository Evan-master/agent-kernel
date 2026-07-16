# X86 Native Mailbox IPC V0 Design

## Status

Implemented and validated on 2026-07-16.

## Purpose

The physical x86 Agents can execute, submit results, complete tasks, and verify
another task, but they cannot yet communicate with one another. This milestone
connects the existing AgentOS mailbox model to the returning ring-3 Agent Call
ABI and proves a complete Send, Receive, and Acknowledge lifecycle between the
two isolated Worker address spaces.

This is native typed Agent IPC. It is not a socket, pipe, byte stream, POSIX
descriptor, shared-memory channel, or host transport.

## Authority Boundary

Mailbox operations are Agent-identity operations rather than resource access.
The core contract therefore continues to require active registered sender and
recipient identities instead of a resource capability. The physical adapter
adds stricter execution checks: every request must match the scheduler-owned
Agent, Task, Image, nonce, running task, admitted launch entry, and private CR3.

Message payload IDs are references only. Sending a `CapabilityId` would not
transfer authority, but V0 deliberately does not expose capability references
through the physical ABI. The only supported native payload field is an
optional `TaskId`; all other `MessagePayload` fields are constructed as `None`.

## Agent Call ABI

ABI version 1 adds operations 7, 8, and 9. Every request retains the trusted
common context in RSI/RDI/R8/R9 and zero flags in RDX.

| Register | SendMessage | ReceiveMessage | AcknowledgeMessage |
| --- | --- | --- | --- |
| `RCX` | 7 | 8 | 9 |
| `R10` | recipient Agent ID | zero | Message ID |
| `R11` | MessageKind code | zero | zero |
| `R12` | optional Task ID, zero for none | zero | zero |
| `R13-R15`, `RBP` | zero | zero | zero |

Message kind codes are 1 Notify, 2 Request, 3 Response, and 4 Fault. Zero and
unknown values are invalid. A SendMessage reply returns the allocated Message
ID in R10. A ReceiveMessage reply returns Message ID, sender Agent ID, kind
code, and optional Task ID in R10-R13. An AcknowledgeMessage reply clears all
operation payload registers. Every reply preserves the trusted context and
nonce and identifies the completed operation in RDX.

All existing operations become canonical across the extended payload
registers: registers unused by an operation must be zero in requests, and every
reply clears them before writing operation-specific values.

## Physical Worker Sequence

Worker A performs four calls:

1. DescribeContext.
2. SubmitTaskResult.
3. SendMessage to Worker B with kind Notify and its own Task ID as payload.
4. CompleteTask.

Worker A compares the returned Message ID with the deterministic first ID. A
mismatch enters its terminal loop before completion.

Worker B performs five calls after it is redispatched:

1. DescribeContext.
2. ReceiveMessage.
3. AcknowledgeMessage.
4. SubmitTaskResult.
5. CompleteTask.

Worker B compares the receive reply with Message ID 1, sender Agent 3, Notify,
and Task 1 before acknowledgement. Any mismatch enters a terminal loop. The
kernel adapter independently validates the same record and all scheduler state;
ring-3 comparisons are evidence, not authority.

## CPU Type States

The sender path is:

`Preempted -> RequestedSenderResult -> AcknowledgedSenderResult -> RequestedMessageSend -> AcknowledgedMessageSend -> CompletedSender`

The receiver path is:

`Preempted -> RequestedMessageReceive -> AcknowledgedMessageReceive -> RequestedMessageAcknowledge -> AcknowledgedMessageAcknowledge -> RequestedReceiverResult -> AcknowledgedReceiverResult -> CompletedReceiver`

CPU modules capture requests, preserve owned frames, encode acknowledged
replies, and count CR3 transitions. Mailbox and task mutations remain in the
boot task adapter and use only public facade syscalls.

## Boot Capacity And Events

`BootedKernel` gains one trailing `MESSAGES` const generic with default zero so
existing callers remain source-compatible. The x86 boot alias opts into one
message record and raises event capacity from 80 to 83.

Verifier setup remains events 38 through 48, and Worker scheduling begins at
event 49. The expected 79-event sequence changes after Worker A's result:

1. Worker A result is event 54, message send is 55, and completion is 56.
2. Worker B dispatch is 57, receive is 58, acknowledgement is 59, result is 60,
   and completion is 61.
3. Verifier queue through completion shifts to events 62 through 69.
4. UART and Driver events shift to events 70 through 79.

At handoff the message is Acknowledged, both Worker results remain stored,
Worker A is Verified with a fulfilled intent, Worker B remains Completed as the
verification control, the Verifier task is Completed, all contexts are Idle,
and the run queue is empty.

## Validation

Host ABI tests cover exact operation codes, canonical request payloads, trusted
context matching, message kind conversion, unsupported payload rejection, and
reply encoding. Boot tests prove the new message capacity is opt-in. QEMU must
prove both physical call sequences, three semantic mailbox events, unchanged
scheduler state around each IPC mutation, exact CR3 transition counts, exactly
79 events, and the existing Verifier and Driver paths.

Debug and release QEMU builds, full workspace tests, Supervisor output, scoped
Clippy, and release disassembly inspection remain required.

## Non-Goals

V0 does not add blocking receive, mailbox wait integration, byte payloads,
capability transfer, message rejection, replies as a special protocol, queues
outside the fixed store, cross-machine transport, encryption, priorities,
timeouts, multicast, or a generic dynamic Agent runtime registry.
