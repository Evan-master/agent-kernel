# Agent Record Retirement V1 Design

## Status

Implemented and locally validated on 2026-07-19. Public `main` publication is
tracked in the adjacent implementation plan.

## Purpose

Every registered Agent permanently occupies one slot in both the dense Agent
Store and its index-aligned execution-context Store. Terminal Task, Runtime,
Entry, Capability, Intent, Admission, and Message records can already return
their fixed capacity, but a retired Agent identity and its idle execution
context remain resident forever.

Agent Record Retirement V1 adds an authenticated lifecycle endpoint for one
retired managed identity. It proves that no live kernel Store still references
the target, removes the Agent Record and execution context atomically, preserves
ordered audit evidence, and makes the paired slot available to a fresh identity.

## Eligibility And Authority

`retire_agent_record(actor, authority, target)` requires:

1. an active registered actor;
2. an existing target Agent Record in `AgentStatus::Retired`;
3. a target with both `manager` and `management_resource` recorded;
4. an idle target execution context with no Task or Driver Invocation;
5. an active, root-scoped Capability held by the actor for the exact management
   Resource;
6. `Operation::Delegate` in the Capability operation set;
7. no remaining non-Event Store reference to the target;
8. one available Event slot.

The original manager has no ambient cleanup authority. A delegated
administrator may retire the record while its complete Capability ancestry
remains active. Trusted bootstrap identities without a management Resource have
no path through this operation.

The architecture adapter additionally requires the target to be absent from
the native runtime registry before Core mutation.

## Strict Reference Preflight

Historical Events do not keep an Agent Record resident. Every other fixed Store
is authoritative liveness state. Retirement rejects a target referenced by:

- another Agent Record as its manager;
- a Resource owner or Capability holder;
- an Intent owner, Task owner or assignee, Run Queue entry, or Runtime
  Admission requester or target;
- an Action, Observation, Checkpoint, Message sender or recipient, Memory Cell
  creator or last writer, Fault, Waiter, Agent Image, or Agent Entry;
- a Namespace owner or `NamespaceObject::Agent` binding;
- a Fault Handler installer or handler, or a Fault Policy installer;
- a Driver Endpoint installer, Driver Binding installer or driver, Driver
  Command driver, or Driver Invocation driver.

The strict policy prevents dangling identity references. Each blocking Store
must complete its own authorized lifecycle before the Agent identity can leave
the registry.

## Retirement High-Water

The Core owns one scalar `retired_agent_floor`. Successful retirement updates
it to `max(current_floor, target.raw())`. Registration first rejects an existing
identity, then rejects zero or any absent identity at or below this floor with
`AgentIdStale`.

This bounded tombstone rule prevents historical Events from aliasing a newly
registered Agent while avoiding an unbounded retired-ID Store. Fresh identities
above the floor may use returned physical capacity. Agent IDs are never changed
or rewritten during dense movement.

## Atomic Dense Removal

The operation performs a read-only preflight before mutation:

1. validate actor and target lifecycle;
2. copy the target Agent Record and aligned execution context;
3. validate managed identity and exact Delegate authority;
4. validate idle execution state;
5. reject every non-Event reference;
6. reserve one Event slot.

After preflight, both arrays shift the same suffix left, both old tail slots are
reset, `agent_len` decreases once, and the retirement floor advances. One Event
is then appended. Every failure preserves both arrays, their order and length,
the retirement floor, and Event length.

## Receipt And Event

`AgentRecordRetirement` contains:

- the complete removed `AgentRecord`;
- the complete removed `AgentExecutionContext`;
- the administrative actor;
- the authorizing Capability;
- the management Resource;
- the resulting retirement floor.

`AgentRecordRetired` records the administrative actor in `agent`, the retired
identity in `target_agent`, the management Resource, the authority Capability,
and `Operation::Delegate`. Earlier lifecycle Events preserve registration,
management, and terminal-state history.

## Facade Contract

`AgentKernel::sys_retire_agent_record(actor, authority, target)` exposes the
Core operation unchanged. Read-only `retired_agent_floor()` inspection is
available through both layers for deterministic validation.

## Agent Call 36

The native ABI adds:

```text
RetireAgentRecord = 36
```

Request payload:

| Register | Value |
| --- | --- |
| `r10` | Delegate authority Capability ID |
| `r11` | retired target Agent ID |
| `r12-r15`, `rbp` | zero |

The scheduler-authenticated Agent, Task, Image, and nonce remain in `rsi`,
`rdi`, `r8`, and `r9`.

Successful reply payload:

| Register | Value |
| --- | --- |
| `r10` | retired target Agent ID |
| `r11` | management Resource ID |
| `r12` | resulting retired Agent floor |
| `r13-r15`, `rbp` | zero |

The architecture executor validates runtime absence, the complete receipt,
paired dense removal, floor advancement, exact Event evidence, and unchanged
running caller context before issuing the canonical reply.

## X86 Proof

The Resource Manager already retires managed Agent 9 and removes its orphaned
Pending Message. It next invokes Agent Call 36 with Capability 12. The Capsule
checks target 9, management Resource 1, and retirement floor 9. It then invokes
Agent Call 17 to register managed Agent 15, proving the returned Agent and
execution-context slot can hold a fresh identity while the stale floor remains
unchanged.

Strict debug and release QEMU validation freezes:

- 33 Resource Manager Agent Calls and 66 Agent/kernel address-space switches;
- one `AgentRecordRetired` Event for Agent 9 followed by one `AgentRegistered`
  Event for Agent 15 at global Events 176 and 177;
- 15 registration Events, 14 final resident Agent records, one retired paired
  slot, one reused paired slot, and retirement floor 9;
- 366 ordered Events through `DriverInvocationCompleted`;
- a 3,075-byte Resource Manager Capsule with SHA-256
  `13a019b701939983d3a90ed938ad4028745e24e448708c5f2dd58d5dbf0f2034`;
- 33 return offsets: `45, 86, 163, 236, 310, 390, 463, 539, 626, 710, 828,
  912, 996, 1080, 1200, 1323, 1449, 1523, 1642, 1735, 1825, 1902, 2048,
  2176, 2253, 2399, 2493, 2621, 2698, 2844, 2958, 3032, 3041`;
- exact equality between independently assembled bytes and the Rust Capsule,
  with one complete occurrence in the release ELF.

## Failure Rules

- unknown actor or target: `AgentNotFound`;
- suspended or retired actor: existing Agent lifecycle error;
- active or suspended target, or busy target context:
  `AgentRecordRetirementNotReady`;
- retired unmanaged target: `AgentManagementDenied`;
- missing, foreign, task-scoped, revoked, attenuated, wrong-Resource, or
  wrong-operation authority: existing Capability authorization error;
- any non-Event target reference: `AgentRecordRetirementReferenced`;
- zero or absent registration identity at or below the retirement floor:
  `AgentIdStale`;
- full Event Log: `EventLogFull` before Store mutation.

## Deferred Work

- kernel-issued Agent identity allocation;
- managed registration for architecture-created Runtime Workers;
- bounded retirement for Agent Images, Resources, Actions, Observations,
  Checkpoints, Memory Cells, Faults, Waiters, and Driver records;
- retirement permits spanning semantic and architecture reclamation;
- durable Event archival and replay checkpoints.
