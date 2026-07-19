# Waiter Compaction V1 Design

## Status

Implemented and frozen on 2026-07-19.

## Purpose

The fixed-capacity Waiter Store retains Signal and Mailbox wait records after
their wake transition marks them inactive. Historical Events already preserve
the wait and wake sequence, yet inactive records continue consuming physical
Store slots and prevent long-running Agents from entering later waits.

Waiter Compaction V1 adds authenticated retirement of one contiguous inactive
prefix. It preserves deterministic FIFO order, monotonic Waiter IDs, complete
replay evidence, and atomic failure while returning physical capacity to both
wait kinds.

## Operation

The Core adds:

```text
compact_waiter_prefix(actor, authority, through)
```

`through` identifies the final Waiter in the selected dense prefix. The
operation retires every record from the current Store head through that ID.
Arbitrary middle deletion remains outside this version so retained order and
bounded mutation cost stay explicit.

## Eligibility And Authority

Compaction requires:

1. an existing active Supervisor entry for `actor`;
2. an existing `through` Waiter ID;
3. every selected Waiter to have `active == false`;
4. active root-scoped `Rollback` authority held by `actor` for every selected
   Waiter Resource;
5. exact authority scope while a selected Resource is active, or active
   ancestor scope while that Resource is retired;
6. one available Event slot per selected Waiter.

The Core uses the shared terminal-metadata cleanup authorization contract for
each record. Mixed-Resource prefixes are allowed only when the same Capability
authorizes every selected Resource under those rules.

Signal and Mailbox wake paths are the only state transitions that deactivate a
Waiter. An active record always blocks the entire requested prefix.

## Reference Model

No live non-Event Core object stores a `WaiterId`. Tasks and architecture
contexts express current waiting state through Task and execution-context
status, while waiter lookup also requires `active == true`. Therefore inactive
status is the complete semantic liveness gate for this version.

Historical Events may retain the Waiter ID after compaction. `next_waiter`
remains monotonic, so a later wait cannot alias that historical identity.
Waiter IDs do not issue generation-bound permits and compaction does not need a
new generation counter.

## Atomic Dense Compaction

The operation performs a read-only preflight before mutation:

1. validate Supervisor identity;
2. locate `through` and compute the selected count;
3. copy and validate every selected record;
4. validate cleanup authority for every selected Resource;
5. reserve aggregate Event capacity.

After preflight, the retained suffix shifts left in stable order, vacated tail
slots reset to `WaiterRecord::empty()`, and `waiter_len` decreases once. One
Event per retired record is appended in original FIFO order. Every failure
preserves records, order, `next_waiter`, and Event length.

## Receipt And Event

`WaiterCompaction` contains:

- the first retired Waiter ID;
- the requested terminal Waiter ID;
- the retired record count.

`EventKind::WaiterCompacted` records:

- the administrative actor in `agent`;
- the waiting Agent in `target_agent`;
- the Waiter Resource and authorizing Capability;
- `Operation::Rollback`;
- the Task, Waiter ID, Signal key, and `WaiterKind`.

The inactive state is implied by the Event kind and eligibility contract. A new
`waiter_kind: Option<WaiterKind>` Event field also labels existing Signal and
Mailbox wait/wake Events whenever they carry a Waiter ID.

## Facade Contract

`AgentKernel::sys_compact_waiter_prefix(actor, authority, through)` exposes the
Core receipt unchanged.

## Agent Call 38

The native ABI adds:

```text
CompactWaiters = 38
```

Request payload:

| Register | Value |
| --- | --- |
| `r10` | Rollback authority Capability ID |
| `r11` | final Waiter ID in the selected prefix |
| `r12-r15`, `rbp` | zero |

The scheduler-authenticated Agent, Task, Image, and nonce remain in `rsi`,
`rdi`, `r8`, and `r9`.

Successful reply payload:

| Register | Value |
| --- | --- |
| `r10` | first retired Waiter ID |
| `r11` | final retired Waiter ID |
| `r12` | retired record count |
| `r13-r15`, `rbp` | zero |

The x86 executor validates the receipt, dense removal, complete ordered Event
evidence, unchanged run queue, and unchanged running caller context before
encoding the canonical reply.

## X86 Capacity And Reuse Proof

The reference x86 profile reduces Waiter capacity from four slots to three.
Before Runtime Admission begins, inactive Waiters 1 and 2 retain the earlier
Signal and Fault Handler waits. The Admission Supervisor creates Mailbox Waiter
3, filling the Store, and receives both first-batch notifications.

The Supervisor then invokes Agent Call 38 through Waiter 3. Three ordered
`WaiterCompacted` Events return all physical slots. During the second batch it
creates monotonic Waiter 4 in a reclaimed slot, proving active capacity reuse
under a profile that could not admit that wait without compaction. After the
second notification round it compacts through Waiter 4, leaving the Store
empty.

The expected frozen profile contains:

- two Waiter compaction Agent Calls;
- four retired Waiter records and four compaction Events;
- one post-compaction Waiter slot reuse;
- 32 Admission Supervisor Agent Calls and 64 address-space switches;
- 374 ordered Events, with first compaction Events 295 through 297, second-wait
  creation Event 300, second compaction Event 331, and Driver completion Event
  374.

## Frozen Evidence

- Admission Supervisor calls: 32;
- Agent/kernel address-space switches: 64;
- machine code: 3,002 bytes;
- complete Capsule: 3,034 bytes;
- Capsule SHA-256:
  `2175ce2b538a6236ce944b4599feacb49e529f6c939d53ae2c5978d26d580ff7`;
- return offsets:
  `44, 82, 169, 247, 361, 400, 495, 609, 648, 766, 889, 976, 1054, 1168,
  1207, 1302, 1416, 1455, 1573, 1693, 1813, 1933, 2053, 2171, 2292, 2410,
  2529, 2647, 2768, 2889, 2971, 3000`;
- independently assembled Capsule equality: exact;
- complete Admission Supervisor Capsule occurrences in the release ELF: one;
- complete Resource Manager Capsule occurrences in the release ELF: one;
- debug and release QEMU traces: exactly 374 ordered Events and successful
  debug-exit status.

## Failure Rules

- missing Supervisor entry: `AgentNotLaunched`;
- non-Supervisor entry: `AgentEntryKindMismatch`;
- unknown terminal ID: `WaiterNotFound`;
- any active selected Waiter: `WaiterCompactionNotReady`;
- missing, foreign, task-scoped, revoked, attenuated, wrong-Resource, or
  wrong-operation authority: existing Capability or Resource error;
- insufficient aggregate Event capacity: `EventLogFull` before Store mutation.

## Deferred Work

- selective inactive Waiter retirement outside the FIFO prefix;
- explicit cancellation of an active wait;
- Fault record retirement and Task `last_fault` cleanup;
- bounded Event archival and replay checkpoints.
