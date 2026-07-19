# Fault Record Retirement V1 Design

## Status

Frozen Evidence on 2026-07-19.

## Purpose

The fixed-capacity Fault Store retains every recovered task fault forever. The
x86 reference profile has four slots and currently fills all four while the
Fault Worker proves `#UD`, `#GP`, write-protection `#PF`, and demand-page
repair. A later task fault therefore fails with `FaultStoreFull` even after the
original Task has completed and its historical evidence is immutable.

Fault Record Retirement V1 adds authenticated compaction of one contiguous
Fault prefix. The transaction clears safe Task `last_fault` references,
preserves deterministic order and monotonic Fault IDs, and records complete
retirement evidence before returning physical Store capacity.

## Operation

The Core adds:

```text
compact_fault_prefix(actor, authority, through)
```

`through` identifies the final Fault in the selected dense prefix. The
operation retires every current record from the Store head through that ID.
Middle deletion remains outside V1 so bounded mutation cost and retained order
remain explicit.

## Eligibility And Authority

Compaction requires:

1. an existing active Supervisor entry for `actor`;
2. an existing `through` Fault ID;
3. no Task in `Faulted` state whose `last_fault` names a selected record;
4. no non-acknowledged Message payload naming a selected record;
5. one shared Capability that authorizes cleanup of every selected Fault
   Resource through the terminal-metadata cleanup contract;
6. one available Event slot per selected Fault.

Cleanup authorization requires active `Rollback` authority held by the actor.
An active target Resource requires exact scope. A retired target Resource
accepts active ancestor scope through its immutable parent chain.

## Reference Model

Task `last_fault` is the only mutable semantic pointer to a Fault record. A
Task in `Faulted` state requires that record for routing, policy application,
and recovery, so compaction rejects the whole prefix. A Task in any other state
may retain the ID only as completed fault history. The transaction clears that
field when it names a selected record.

Message payloads can also carry a Fault ID. `Pending` and `Received` Messages
remain live delivery state and block compaction. An `Acknowledged` Message is
terminal immutable evidence and follows the same historical-reference rule
already used by Task and Intent compaction.

Events may retain Fault IDs after compaction. `next_fault` remains monotonic,
so a later fault cannot alias any historical Event or acknowledged Message.
Fault IDs issue no generation-bound permits.

## Atomic Dense Compaction

The operation completes a read-only preflight before mutation:

1. validate Supervisor identity;
2. locate `through` and compute the selected count;
3. copy and validate every selected Fault record;
4. validate Task and Message references;
5. validate cleanup authority for every selected Resource;
6. reserve aggregate Event capacity.

After preflight, every safe Task `last_fault` that names the selected prefix is
cleared. The retained Fault suffix shifts left in stable order, vacated tail
slots reset to `FaultRecord::empty()`, and `fault_len` decreases once. One Event
per retired record is appended in original order. Every failure preserves Task
records, Fault records, `next_fault`, and Event length.

## Receipt And Event

`FaultCompaction` contains:

- the first retired Fault ID;
- the requested terminal Fault ID;
- the retired record count.

`EventKind::FaultCompacted` records:

- the administrative actor in `agent`;
- the faulting Agent in `target_agent`;
- the Fault Resource and authorizing Capability;
- `Operation::Rollback`;
- the Task, Fault ID, `FaultKind`, and detail value.

## Facade Contract

`AgentKernel::sys_compact_fault_prefix(actor, authority, through)` exposes the
Core receipt unchanged.

## Agent Call 39

The native ABI adds:

```text
CompactFaults = 39
```

Request payload:

| Register | Value |
| --- | --- |
| `r10` | Rollback authority Capability ID |
| `r11` | final Fault ID in the selected prefix |
| `r12-r15`, `rbp` | zero |

The scheduler-authenticated Agent, Task, Image, and nonce remain in `rsi`,
`rdi`, `r8`, and `r9`.

Successful reply payload:

| Register | Value |
| --- | --- |
| `r10` | first retired Fault ID |
| `r11` | final retired Fault ID |
| `r12` | retired record count |
| `r13-r15`, `rbp` | zero |

The x86 executor snapshots all four bounded Fault records, invokes the public
facade, and validates the receipt, Task-reference cleanup, dense removal,
complete ordered Events, unchanged run queue, and unchanged running caller
context before encoding the canonical reply.

## X86 Full-Store Recovery Proof

The reference x86 profile retains its four-slot Fault capacity. The existing
Fault Worker fills the Store with Faults 1 through 4. During the later resident
Supervisor flow, Task compaction first removes the terminal Task prefix that
contains the Fault Worker. Agent Call 39 then compacts through Fault 4.

The validated profile contains:

- one Fault compaction Agent Call;
- four retired Fault records and four `FaultCompacted` Events;
- an empty final Fault Store after starting from full capacity;
- 33 Admission Supervisor Agent Calls and 66 address-space switches;
- 378 ordered Events, with Fault compaction Events 340 through 343 and Driver
  completion Event 378.

The Admission Supervisor Capsule contains 3,122 code bytes and 3,154 total
bytes. Its return offsets are:

```text
44, 82, 169, 247, 361, 400, 495, 609, 648, 766, 889, 976, 1054,
1168, 1207, 1302, 1416, 1455, 1573, 1693, 1813, 1933, 2053, 2173,
2291, 2412, 2530, 2649, 2767, 2888, 3009, 3091, 3120
```

The complete Capsule SHA-256 is
`260768b39dec9e14d895ae0ebf14972114f7ccfb7658058d956df8d99dc3e527`.
Independent assembly matches the embedded Rust bytes exactly. The complete
Admission Supervisor Capsule and the unchanged Resource Manager Capsule each
occur exactly once in the release ELF. Debug and release QEMU both reproduce
the strict 378-Event sequence and all marker counts.

## Failure Rules

- missing Supervisor entry: `AgentNotLaunched`;
- non-Supervisor entry: `AgentEntryKindMismatch`;
- unknown terminal ID: `FaultNotFound`;
- selected Fault still required by a Faulted Task: `FaultCompactionNotReady`;
- selected Fault referenced by a live Message: `FaultCompactionReferenced`;
- missing, foreign, task-scoped, revoked, attenuated, wrong-Resource, or
  wrong-operation authority: existing Capability or Resource error;
- insufficient aggregate Event capacity: `EventLogFull` before mutation.

## Deferred Work

- selective Fault retirement outside the FIFO prefix;
- Fault Handler and Fault Policy uninstall lifecycles;
- a second physical trap sequence that consumes reclaimed Fault slots;
- bounded Event archival and replay checkpoints.
