# Agent Entry Retirement V1 Design

## Status

Accepted for implementation on 2026-07-19.

## Purpose

An Agent Entry is the active launch binding between an Agent identity, one
verified image, execution authority, a Resource, and an optional Task or
Intent scope. Completed native executions currently leave this binding in the
fixed-capacity Entry Store forever. The retained Entry blocks Capability
retirement and prevents the same active Agent identity from receiving a later
launch binding.

This milestone adds authenticated retirement for one quiescent Agent Entry.
Retirement removes runtime authority from the active Entry Store while
preserving the original launch and the exact retirement decision in ordered
Events. Agent records, images, terminal Tasks, terminal Admissions, and
historical messages remain independent objects.

## Identity And Store Behavior

Entries have no separate numeric ID. The active Agent ID is their unique key,
and at most one Entry may exist for an Agent.

`retire_agent_entry(actor, authority, target)` accepts one target Agent:

- the target Entry must exist;
- the target Agent record remains registered;
- the Entry Store removes the target and shifts later records left without
  changing any retained record;
- the vacated tail is reset to `AgentEntryRecord::empty()`;
- `agent_entry(target)` returns `AgentEntryNotFound` after retirement;
- `agent_entry_count()` decreases by one while `agent_entry_capacity()` stays
  fixed;
- a still-active target Agent may later receive a fresh launch binding.

No Entry generation or prepare/commit permit is introduced in V1. Every
operation that depends on an Entry performs a current lookup at commit time.

## Caller And Authority

The actor must be active and hold a launched `Supervisor` Entry. The authority
must be an active, root-scoped Capability carrying `Rollback`.

For an Entry on an active Resource, authority scope must match that Resource
exactly. For an Entry on a retired Resource, authority may belong to an active
ancestor Resource. The immutable parent chain is bounded by Resource Store
capacity. Capability Store compaction and Agent Entry retirement share this
cleanup-authority rule.

An executing Supervisor cannot retire its own Entry because its execution
context is not quiescent.

## Terminal Scope

The target execution context must be `Idle`, with no Task or Driver Invocation
attached.

For a Task-scoped Entry:

- a retained Task must belong to the target and match the Entry Resource;
- its state must be `Completed`, `Verified`, or `Cancelled`;
- a missing Task is accepted because Task prefix compaction only removes
  verified/fulfilled or cancelled/cancelled terminal pairs.

For an Entry with an Intent and no Task:

- a retained Intent must belong to the target and match the Entry Resource;
- its state must be `Fulfilled` or `Cancelled`;
- a missing Intent is accepted after authenticated Intent compaction.

An Entry with no Task and no Intent requires the target Agent record to be
`Retired`. This keeps bootstrap and open-ended service launches alive until an
explicit Agent lifecycle transition closes them.

## Live References

Retirement rejects every object that can still dispatch, resume, or route work
through the launch binding:

- a Run Queue item for the target Agent;
- an active Waiter for the target Agent;
- a `Requested` or `Admitted` Runtime Admission where the target is either the
  requester or admitted Agent;
- a `Received` and unacknowledged Message held by the target;
- any installed Fault Handler naming the target as handler;
- any Driver Binding naming the target as driver.

Pending mailbox messages remain Agent-identity state and can be consumed by a
later launch. Acknowledged Messages, terminal Runtime Admissions, completed
Driver or Fault records, Events, Actions, Observations, Checkpoints, Images,
and terminal Tasks are historical and do not keep an Entry active.

The architecture handler additionally requires the target to be absent from
the native runtime registry. Core execution state and architecture-owned CPU
or address-space ownership therefore agree before mutation.

## Atomic Mutation

Retirement performs a read-only preflight:

1. authenticate the launched Supervisor;
2. locate and copy the target Entry;
3. validate terminal scope and quiescent execution;
4. validate exact-Resource or retired-descendant cleanup authority;
5. reject every live reference class;
6. reserve one Event slot.

After preflight, the dense store removes exactly one Entry and records exactly
one `AgentEntryRetired` Event. Every failure preserves Entry order, count,
Event length, and all referenced stores.

## Receipt And Event

`AgentEntryRetirement` is a copyable receipt carrying the retired
`AgentEntryRecord`. It exposes the complete immutable record without a pointer
or mutable store reference.

`AgentEntryRetired` records:

- the retiring Supervisor in `agent`;
- the target Agent in `target_agent`;
- the retired Entry Capability in `capability`;
- cleanup authority in `source_capability`;
- `Rollback` in `operation`;
- the Entry Resource, Image, mapped image kind, optional Intent, and optional
  Task;
- the next global Event sequence.

The earlier `AgentLaunched`, image, Capability, Intent, and Task Events retain
the complete creation lineage.

## Agent Call 33

The native ABI adds:

```text
RetireAgentEntry = 33
```

Request payload:

| Register | Value |
| --- | --- |
| `r10` | cleanup authority Capability ID |
| `r11` | target Agent ID |
| `r12-r15`, `rbp` | zero |

The authenticated Agent, Task, Image, and nonce remain in `rsi`, `rdi`, `r8`,
and `r9`.

Successful reply payload:

| Register | Value |
| --- | --- |
| `r10` | retired target Agent ID |
| `r11-r15`, `rbp` | zero |

Malformed, unauthenticated, nonterminal, referenced, unauthorized, native-live,
and Event-capacity failures fail closed.

## X86 Proof

The resident Admission Supervisor calls operation 33 for first-batch Runtime
Service Agents 10 and 11 after their Tasks are verified, private address spaces
are reclaimed, native runtime contexts are removed, and Runtime Admissions 1
and 2 are compacted.

Both Entries use active root Resource 1 and are retired through Capability 23
with exact `Rollback` scope. Their terminal Task records remain available,
their launch Capabilities remain present, and all other Entry records retain
their exact values and order. The active Entry count falls by two.

The proof adds two consecutive `AgentEntryRetired` Events before Capability
Store lifecycle evidence. The Supervisor transcript grows by two Agent Calls
and four Agent/kernel address-space switches.

Expected proof markers:

```text
AGENT_KERNEL_AGENT_CALL_AGENT_ENTRY_RETIREMENT_OK
AGENT_KERNEL_NATIVE_AGENT_ENTRY_RETIREMENT_OK
```

The independently assembled artifact freezes these values:

- machine code: 2368 bytes;
- complete Capsule: 2400 bytes;
- SHA-256: `67479c274f350fa4bdf625c86fa4d9dc2c2a0af643b1a22b3d44aad160bd2a71`;
- return offsets: `44, 82, 169, 247, 358, 395, 506, 572, 659, 737, 848,
  885, 996, 1059, 1179, 1299, 1419, 1537, 1658, 1776, 1895, 2013, 2134,
  2255, 2337, 2366`;
- Supervisor transcript: 26 Agent Calls and 52 Agent/kernel address-space
  switches;
- retirement Events: sequences 331 and 332;
- final Event count: 358.

The complete Capsule occurs exactly once in the validated release ELF.

## Failure Rules

- Unknown target returns `AgentEntryNotFound`.
- Nonterminal scope or busy execution returns
  `AgentEntryRetirementNotReady`.
- Queue, Waiter, Admission, Message, Fault Handler, or Driver Binding
  references return `AgentEntryRetirementReferenced`.
- Worker callers fail the Supervisor-entry check.
- Missing, revoked, foreign, task-scoped, attenuated, or wrongly scoped
  authority returns the existing Capability or Resource error.
- Event exhaustion returns `EventLogFull` before mutation.
- Architecture-owned native liveness fails closed before calling the facade.

## Deferred Work

- Agent identity compaction and execution-context slot reuse;
- Fault Handler uninstall and Driver unbind lifecycle operations;
- retirement permits spanning architecture reclamation transactions;
- durable audit export and replay checkpoints;
- policy for pending Agent mailbox state during identity retirement.
