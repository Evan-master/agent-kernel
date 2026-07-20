# Native Namespace Memory Transport V4 Design

Status: Implemented, verified, and published on 2026-07-20

## Objective

Carry three- and four-hop Namespace paths across the native ring-3 boundary
without accepting arbitrary userspace pointers. Every Agent receives one fixed,
private call-data page. Agent Call 51 snapshots one canonical 112-byte record
from the page and passes the decoded path to the existing bounded Core resolver.

## Ownership Boundaries

- `agent-kernel-core` retains authority checks, mount traversal, Event
  atomicity, and the maximum path depth of four.
- `agent-kernel` retains the architecture-neutral syscall facade.
- The `agent-kernel-x86_64` library owns the fixed virtual address, canonical
  byte format, pure decoder, Agent Call 51 register contract, and host tests.
- The x86 bare-metal binary owns the physical frame, page-table mapping,
  supervisor alias, volatile snapshot, execution checks, and QEMU evidence.
- The ring-3 Resource Manager writes one four-hop request and invokes Call 51.

No arbitrary address, variable length, allocator, page fault, host parser, or
ambient authority enters the call path.

## Fixed Call-Data Page

`UserMemoryLayout` places the page immediately after the reserved runtime-region
range. Existing code, signal, guard, stack, lazy-data, runtime-page, and runtime-
region addresses remain stable.

The mapping is:

```text
ring 3 : present | user | writable | NX
ring 0 : absent from the kernel virtual Agent region
alias  : supervisor physical window, owned by PreparedAgentMemory
```

The page receives its own physical frame and becomes part of
`AgentMemoryIdentity`. Preparation and restart clear all 4 KiB. Address-space
reclamation preserves exclusive frame ownership.

The signal page remains read-only to ring 3. Agent-controlled request bytes
cannot forge call release, quantum generation, or restart generation.

## Canonical Wire Record

All words use little-endian `u64` encoding.

| Offset | Bytes | Field |
| ---: | ---: | --- |
| `0` | 8 | magic: `NSPATH51` |
| `8` | 8 | format version: `1` |
| `16` | 8 | nonzero request generation |
| `24` | 8 | root Workspace ID |
| `32` | 8 | depth: `3` or `4` |
| `40` | 8 | canonical record length: `112` |
| `48` | 16 | segment 1: Observe Capability, key |
| `64` | 16 | segment 2: Observe Capability, key |
| `80` | 16 | segment 3: Observe Capability, key |
| `96` | 16 | segment 4: Observe Capability, key |

Every active segment requires a nonzero Capability ID. A depth-three record
requires both words in segment 4 to be zero. Root and generation must match the
register envelope. Any mismatch rejects the call before Core mutation or Event
allocation.

## Agent Call 51

`ResolveNamespacePathFromMemory = 51` uses the common authenticated identity
envelope and the following operation payload:

| Register | Value |
| --- | --- |
| `r10` | root Workspace ID |
| `r11` | nonzero request generation |
| `r12` | record length, exactly `112` |
| `r13` | zero |
| `r14` | zero |
| `r15` | zero |
| `rbp` | zero |

Register decoding and scheduled-context authentication happen before the page
snapshot. The single active CPU has already stopped ring 3 and switched to the
kernel CR3, so the Agent cannot mutate its page during the bounded copy. SMP
support will add cross-CPU ownership synchronization before sharing an Agent
address space.

The reply uses the complete terminal Namespace Entry record and operation ID
51. The executor validates all four pre-resolution records, all four ordered
`NamespaceEntryResolved` Events, unchanged Namespace occupancy, and the
terminal receipt before resuming ring 3.

## Native Proof

The Resource Manager establishes this chain with independent Observe
Capabilities:

```text
Workspace 1 -> Mount(Workspace A)
Workspace A -> Mount(Workspace B)
Workspace B -> Mount(Workspace C)
Workspace C -> terminal object
```

It writes the canonical record to its fixed call-data page, invokes Call 51,
and validates the returned terminal record. Existing one/two-hop Call 50 proof
remains intact.

The final proof must establish:

- the call-data virtual page is stable, user-writable, NX, kernel-region
  unmapped, and backed by an Agent-exclusive content frame;
- malformed magic, version, generation, root, depth, length, authority, and
  unused-slot encodings fail in host contracts;
- Call 51 rejects noncanonical registers and authenticates scheduled identity;
- the four-hop QEMU path emits exactly four ordered resolution Events;
- debug and release QEMU transcripts, Capsule digests, return offsets, and
  Release ELF occurrences agree with checked evidence.

## Verified Profile

| Verified artifact | Value |
| --- | --- |
| Call-data virtual page | `0x0000400000011000`, user RW, NX |
| Agent-owned frames | `12` (`4` page-table + `8` content) |
| Resource / Capability / Namespace capacity | `10 / 30 / 4` |
| Live Event capacity / archived Events | `371 / 64` |
| Resource Manager Calls / CR3 switches | `42 / 84` |
| Resource Manager Capsule | `3,984` bytes |
| Resource Manager SHA-256 | `871151ca85099c942d442af1f4bc01b898e6a3ed85bfda73c76839cb612f73b8` |
| Admission Supervisor Calls / CR3 switches | `44 / 88` |
| Admission Supervisor Capsule | `4,115` bytes |
| Admission Supervisor SHA-256 | `4abda1fd30408ce5e24f1ce19dba523c04d3edc6bde2dc6ee014414ff45662dd` |
| Final live Events / next sequence | `341 / 406` |
| Debug / Release transcript | Events `1..405`, `SUPERVISOR_HANDOFF_READY` |
| Release ELF occurrence | each Capsule `1`, each code body `1` |

The exact V4 Namespace window is:

```text
event[185] namespace_entry_bound
event[186] namespace_entry_bound
event[187] namespace_entry_resolved
event[188] namespace_entry_resolved
event[203] resource_created
event[204] capability_granted
event[205] resource_created
event[206] capability_granted
event[207] namespace_entry_rebound
event[208] namespace_entry_bound
event[209] namespace_entry_bound
event[210] namespace_entry_resolved
event[211] namespace_entry_resolved
event[212] namespace_entry_resolved
event[213] namespace_entry_resolved
```

## Verification Gates

- pure decoder and Call 51 host contracts;
- fixed layout, frame identity, reclamation, restart, and mapping contracts;
- full Workspace tests and Supervisor simulation;
- `no_std`, strict Clippy, and bare-metal compile;
- debug and release QEMU transcript validation;
- independent assembly, digest, return-offset, and Release ELF audits.

## Deferred Work

- generalized typed call-data messages beyond Namespace paths;
- SMP snapshot ownership and cross-CPU revocation synchronization;
- Namespace path mutation and mount delegation receipts;
- durable signed Namespace snapshots and distributed generation proofs.
