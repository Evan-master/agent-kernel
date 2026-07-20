# Typed Namespace Path Rebind V5 Design

Status: Implemented and verified; publication pending on 2026-07-21

## Objective

Turn the fixed Agent call-data page into a typed native message transport and
use its first message kind to perform an optimistic Namespace mutation through
a bounded path. The kernel must authenticate every mount hop, require `Act` at
the terminal Workspace, compare the terminal revision, validate the replacement
object, and commit one ordered Event transaction.

## Ownership Boundaries

- `agent-kernel-core` owns traversal authority, revision comparison, object
  validation, mount-cycle rejection, state mutation, and Event atomicity.
- `agent-kernel` exposes the architecture-neutral syscall facade.
- The `agent-kernel-x86_64` library owns the typed wire envelope, pure decoder,
  Call 52 register contract, and canonical reply.
- The x86 bare-metal binary owns the authenticated fixed-page snapshot,
  executor reconciliation, Capsule proof, and QEMU transcript.
- The ring-3 Resource Manager writes one typed four-hop rebind request and
  validates the operation, terminal Entry ID, replacement object, and revision.
  The kernel executor reconciles the complete resulting Namespace Entry record.

The message carries no address, variable allocation, ambient identity, host
parser, or mutable kernel pointer.

## Core Transaction

`compare_and_rebind_namespace_path` accepts an actor, root Workspace, one to
four path segments, expected terminal revision, and replacement object.

For every nonterminal segment it requires `Observe`, resolves one Mount, checks
the target Workspace, and rejects repeated Workspaces. The terminal segment
requires `Act`, resolves the target Entry by key, compares its revision, and
validates the replacement with the existing Namespace object and mount-cycle
rules.

All checks and Event-capacity preflight complete before mutation. A depth `N`
success emits `N - 1` ordered `NamespaceEntryResolved` Events followed by one
`NamespaceEntryRebound` Event. The return receipt contains the root, previous
terminal record, resulting terminal record, and depth.

## Typed Call-Data Record

All words are little-endian `u64`. The first message kind has a canonical size
of 160 bytes.

| Offset | Bytes | Field |
| ---: | ---: | --- |
| `0` | 8 | magic: `AGNTMSG1` |
| `8` | 8 | envelope version: `1` |
| `16` | 8 | nonzero request generation |
| `24` | 8 | message kind: `1` (`CompareAndRebindNamespacePath`) |
| `32` | 8 | total record length: `160` |
| `40` | 8 | payload length: `112` |
| `48` | 16 | segment 1: Capability, key |
| `64` | 16 | segment 2: Capability, key |
| `80` | 16 | segment 3: Capability, key |
| `96` | 16 | segment 4: Capability, key |
| `112` | 8 | root Workspace ID |
| `120` | 8 | depth: `1..4` |
| `128` | 8 | nonzero expected terminal revision |
| `136` | 8 | canonical replacement Namespace object |
| `144` | 8 | flags: zero |
| `152` | 8 | reserved: zero |

Every active segment requires a nonzero Capability. Every unused segment must
be all zero. The decoder rejects unknown kinds, envelope mismatches, malformed
objects, noncanonical flags, invalid depth, generation mismatch, and unused
data. The expected terminal revision rejects a replay after a successful
mutation.

## Agent Call 52

`CompareAndRebindNamespacePathFromMemory = 52` uses the common scheduled
identity envelope and this operation payload:

| Register | Value |
| --- | --- |
| `r10` | nonzero request generation |
| `r11` | zero |
| `r12` | zero |
| `r13` | zero |
| `r14` | zero |
| `r15` | zero |
| `rbp` | zero |

Register decoding and scheduled-context authentication precede the fixed-page
snapshot. Ring 3 is stopped and the single CPU has switched to the kernel CR3
during the bounded copy. The reply uses operation ID 52 and the complete
resulting terminal Entry record.

## Native Proof

The existing Resource Manager chain remains:

```text
Workspace 1 -> Workspace 3 -> Workspace 8 -> Workspace 9 -> terminal Entry
```

Call 51 first resolves the four-hop path. The Resource Manager then writes a
typed message with generation `1`, expected terminal revision `1`, and a
replacement `Resource(3)`. Call 52 must advance Entry 4 to revision `2`, emit
three ordered resolution Events and one terminal rebind Event, preserve
Namespace occupancy, and return the resulting record to ring 3. Ring 3 checks
the operation, Entry ID, object, and revision; executor evidence checks every
field in the record and receipt.

## Verification Gates

- Core success, late-authority, stale-revision, invalid-object, mount-cycle,
  empty/oversized-path, and Event-capacity contracts;
- facade forwarding and receipt contracts;
- typed envelope, canonical payload, register decode, authentication, and reply
  contracts;
- fixed-page snapshot and executor reconciliation;
- exact debug/release QEMU transcript, marker counts, call counts, return
  offsets, Capsule digests, and Release ELF occurrences;
- Workspace tests, Supervisor simulation, `no_std`, formatting, and strict
  Clippy.

## Verified Profile

| Verified artifact | Value |
| --- | --- |
| Typed call-data record | `160` bytes, message kind `1` |
| Native operation | Agent Call `52` |
| Call-data virtual page | `0x0000400000011000`, user RW, NX |
| Resource / Capability / Namespace capacity | `10 / 30 / 4` |
| Live Event capacity / archived Events | `375 / 64` |
| Resource Manager Calls / CR3 switches | `43 / 86` |
| Resource Manager code / Capsule | `4,093 / 4,125` bytes |
| Resource Manager SHA-256 | `0bd528c2ae19772d3810ba54018035ff98d72ef03666dfe5872f4c0211a12c50` |
| Admission Supervisor Calls / CR3 switches | `44 / 88` |
| Admission Supervisor Capsule | `4,115` bytes |
| Admission Supervisor SHA-256 | `4abda1fd30408ce5e24f1ce19dba523c04d3edc6bde2dc6ee014414ff45662dd` |
| Final live Events / next sequence | `345 / 410` |
| Debug / Release transcript | Events `1..409`, `SUPERVISOR_HANDOFF_READY` |
| Release ELF occurrence | each Capsule `1`, each code body `1` |
| Workspace tests | `216` result groups, `742` passed tests |

The V5 mutation window is:

```text
event[210] namespace_entry_resolved  # Call 51 / hop 1
event[211] namespace_entry_resolved  # Call 51 / hop 2
event[212] namespace_entry_resolved  # Call 51 / hop 3
event[213] namespace_entry_resolved  # Call 51 / terminal
event[214] namespace_entry_resolved  # Call 52 / mount 1
event[215] namespace_entry_resolved  # Call 52 / mount 2
event[216] namespace_entry_resolved  # Call 52 / mount 3
event[217] namespace_entry_rebound   # revision 2 / Resource(3)
```

## Deferred Work

- multi-page Capsule packaging or a production loader; the V5 Resource Manager
  code occupies `4,093 / 4,096` bytes;
- additional typed message kinds for bulk Resource, Capability, and Memory
  operations;
- typed replies larger than the canonical register frame;
- kernel-tracked monotonic generations for future message kinds without an
  optimistic revision guard;
- SMP snapshot ownership and cross-CPU address-space synchronization;
- durable signed Namespace transactions and distributed generations.
