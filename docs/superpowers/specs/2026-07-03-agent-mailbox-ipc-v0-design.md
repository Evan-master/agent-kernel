# Agent Mailbox IPC V0 Design

## Purpose

Agent Mailbox IPC V0 adds a native kernel communication primitive for agents.
It is not a socket, file descriptor, pipe, or process message queue. It is a
fixed-capacity, deterministic kernel store of typed messages between registered
agents.

## Scope

V0 provides:

- first-class `MessageId`, `MessageRecord`, `MessageKind`, `MessagePayload`, and
  `MessageStatus` types,
- fixed-capacity message storage owned by `KernelCore`,
- `send_message(sender, recipient, kind, payload)` for active agents,
- `receive_message(agent)` for FIFO delivery of pending messages addressed to an
  active agent,
- `acknowledge_message(agent, message)` for closing received messages,
- replayable `MessageSent`, `MessageReceived`, and `MessageAcknowledged` events,
- facade syscalls and read-only message inspection.

V0 intentionally does not provide byte streams, heap-allocated payloads, host
networking, shared memory, priorities, blocking waits, timers, or encryption.

## Core Model

```rust
pub struct MessageId(u64);

pub enum MessageKind {
    Notify,
    Request,
    Response,
}

pub enum MessageStatus {
    Pending,
    Received,
    Acknowledged,
}

pub struct MessagePayload {
    pub resource: Option<ResourceId>,
    pub capability: Option<CapabilityId>,
    pub intent: Option<IntentId>,
    pub task: Option<TaskId>,
    pub action: Option<ActionId>,
}

pub struct MessageRecord {
    pub id: MessageId,
    pub sender: AgentId,
    pub recipient: AgentId,
    pub kind: MessageKind,
    pub payload: MessagePayload,
    pub status: MessageStatus,
}
```

`KernelCore` gains an explicit `MESSAGES` capacity, a fixed message array, a
message length, and a deterministic `next_message` counter.

## Authority And Ordering

Sending requires both sender and recipient to be active registered agents.
Receiving and acknowledging require the actor to be active and to match the
message recipient. Suspended, retired, or unknown agents fail before message
lookup, queue lookup, capacity checks, or event mutation.

`receive_message(agent)` returns the oldest `Pending` message for that recipient
by store order. It marks the message `Received` and records
`MessageReceived`. If no pending message exists, it returns `MailboxEmpty`
without mutating state.

`acknowledge_message(agent, message)` requires the message to be addressed to
the agent and already `Received`. It marks the message `Acknowledged` and
records `MessageAcknowledged`.

Every successful mutating mailbox operation appends exactly one event. Capacity
or status failures leave messages and events unchanged.

## Test Evidence

Tests must prove:

- sending records a pending message and `MessageSent`,
- receive delivers FIFO pending messages and records `MessageReceived`,
- acknowledge closes a received message and records `MessageAcknowledged`,
- store-full and event-log-full failures are atomic,
- unknown, suspended, or retired senders and recipients are rejected,
- receive on an empty mailbox returns `MailboxEmpty`,
- wrong-recipient acknowledgement returns `MessageAgentMismatch`,
- acknowledging a pending message returns `MessageStatusMismatch`,
- facade syscalls expose the same behavior through `AgentKernel`.
