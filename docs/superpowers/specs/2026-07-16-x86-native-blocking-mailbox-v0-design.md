# X86 Native Blocking Mailbox V0 Design

## Status

Implemented, validated, merged, and published on 2026-07-16.

## Purpose

Native mailbox calls currently succeed only when the receiver runs after a
message already exists. This milestone makes an empty `ReceiveMessage` a real
kernel scheduling boundary: the receiving task becomes Waiting, its captured
ring-3 call frame remains owned by the kernel, a sender wakes it by creating a
message, and the original call later returns from the same frame.

This is Agent-native blocking IPC. It is not a thread park, condition variable,
socket readiness API, host callback, polling loop, or async runtime.

## Core Wait Model

The existing fixed-capacity waiter store is extended with `WaiterKind`:

- `Signal` retains the existing resource and signal-key behavior.
- `Mailbox` identifies a task blocked on its assigned Agent mailbox.

`WaiterRecord` keeps its task, Agent, task resource, active bit, and deterministic
ID. Mailbox waiters use `SignalKey(0)` only as unused storage; `WaiterKind`
provides the semantic distinction.

The core adds:

```rust
receive_or_wait_message(agent, task_capability, task) -> MessageReceiveOutcome
```

If a pending message already exists, the call performs the existing FIFO receive
transition and returns `Received(MessageId)`. If the mailbox is empty, it
requires the caller to be the admitted assignee of a Running task with matching
task-scoped Act authority, allocates one mailbox waiter, changes the task and
execution context to Waiting, records `MessageWaitStarted`, and returns
`Waiting(WaiterId)`.

`send_message` keeps its existing return type. When the recipient has an active
mailbox waiter, it atomically reserves message, run-queue, and two event slots;
records `MessageSent`; deactivates the waiter; changes the task to Accepted;
enqueues it; and records `MessageWaitWoken`. Capacity or validation failure
leaves the message store, waiter, task, execution context, run queue, counters,
and events unchanged. Sending to a recipient without a mailbox waiter retains
the exact one-event V0 behavior.

## Event Contract

`MessageWaitStarted` carries the waiting Agent, task capability, task resource,
task, intent, and waiter. `MessageWaitWoken` carries the sender, recipient,
waiting task resource, task, intent, waiter, and newly allocated Message ID. It
does not claim capability use because Agent-to-Agent sending remains an identity
operation.

Signal wait events and semantics remain unchanged. Event labels are
`message_wait_started` and `message_wait_woken`.

## Physical Schedule

Worker identities and immutable Capsules remain unchanged, but initial FIFO
order becomes B then A:

1. Dispatch B, preempt it once, expire its quantum, and dispatch A.
2. Preempt A once, expire its quantum, and redispatch B.
3. Resume B through DescribeContext and ReceiveMessage. With no pending message,
   retain the captured receive frame and move B to Waiting.
4. Dispatch A while B's frame remains kernel-owned.
5. Resume A through DescribeContext, SubmitTaskResult, SendMessage, and
   CompleteTask. Send wakes and enqueues B; completion dispatches B.
6. Receive the pending message on behalf of B's blocked call, encode the reply
   into the retained frame, then resume B through AcknowledgeMessage,
   SubmitTaskResult, and CompleteTask.

The sender still performs four calls and eight Agent-call CR3 transitions. The
receiver still performs five calls and ten transitions; its receive call simply
spans an intervening physical Agent execution and scheduler wait/wake cycle.

## Semantic Type States

The receiver CPU path adds `WaitingMessageReceiveCpu`, which can only be created
after a matching core mailbox waiter exists. It owns the original
`RequestedMessageReceiveCpu` and its saved privilege frame. Only a validated
message wake and redispatch can consume it to encode a ReceiveMessage reply.

The boot task flow explicitly models:

`ReceiverRunning -> SenderRunning -> ReceiverResumed -> ReceiverWaiting -> SenderResumed -> MessageSent/ReceiverWoken -> ReceiverRedispatched -> MessageReceived`

Task and execution-context state is checked around every physical or semantic
transition. CPU helpers never mutate the core; semantic adapters use only facade
syscalls.

## Boot Capacity And Events

`BootedKernel` gains a trailing `WAITERS` const generic with default zero. The
x86 alias opts into one waiter and raises event capacity from 83 to 86.

Setup remains events 1 through 48. The exact expected boot tail is:

1. B dispatch, B expiry, A dispatch, A expiry, B dispatch: events 49-53.
2. B wait and A dispatch: events 54-55.
3. A result, send, B wake, A completion, B dispatch: events 56-60.
4. B receive, acknowledgement, result, completion: events 61-64.
5. Verifier queue through completion: events 65-72.
6. UART and Driver flow: events 73-82.

At handoff the mailbox waiter is inactive, the message is Acknowledged, both
Workers retain their results, Worker A alone is Verified, all execution contexts
are Idle, and the run queue is empty.

## Validation

Core tests cover immediate receive, empty-mailbox wait, send wakeup, waiter FIFO,
authority, waiter capacity, run-queue capacity, event capacity, and atomic
failure. Facade tests cover both receive outcomes. x86 tests cover the retained
receive token and trusted context.

Debug and release QEMU must emit the blocking wait/wake markers, preserve the
existing mailbox, Verifier, and Driver markers, and produce exactly 82 ordered
events. Full workspace tests, Supervisor output, no_std checks, scoped Clippy,
and release disassembly inspection remain required.

## Non-Goals

V0 does not add multiple simultaneous waits per Agent, timeout or cancellation,
select over several mailboxes, message rejection, priorities, broadcast wake,
cross-machine transport, byte payloads, capability transfer, SMP wakeups, or a
general dynamic native Agent runtime registry.
