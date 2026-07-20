# Native Namespace Hierarchy V3 Design

Status: Implemented and verified

## Objective

Add explicit Workspace mounts and bounded hierarchical resolution to the native
Namespace. A mount is a distinct kernel object, so an ordinary Resource binding
never acquires traversal semantics implicitly.

Every path hop carries its own Observe Capability. Core validates the complete
path before allocating any Event, then records one `NamespaceEntryResolved`
Event per hop. Failed traversal leaves the Event Log and all Stores unchanged.

## Ownership Boundaries

- `agent-kernel-core` owns mount validity, cycle prevention, bounded traversal,
  authority order, receipts, and Event atomicity.
- `agent-kernel` exposes the architecture-neutral syscall-style path API.
- `agent-kernel-x86_64` owns Agent Call 50, the two-hop register contract,
  canonical replies, native execution checks, and QEMU evidence.
- The ring-3 Resource Manager proves a real two-Workspace path with independent
  root and child Capabilities.

No allocator, userspace pointer, host path parser, ambient authority, or POSIX
directory rule enters the protocol.

## Mount Object

Core adds one explicit Namespace object:

```text
NamespaceObject::Mount(ResourceId)
```

The target must be an active `Workspace`. Bind, force-rebind, and
compare-and-rebind all reject:

- a target with another Resource kind;
- a retired or missing target;
- a direct self-mount;
- an edge that makes the source Workspace reachable from the target.

Cycle detection scans only the fixed Namespace Store. The mutation under
consideration is excluded during rebind validation, and no heap or recursion is
used. `NamespaceMountCycle` reports every rejected cycle.

Mount targets remain live Resource references for record-retirement checks.
`NamespaceObject::Resource` remains an opaque object reference.

## Bounded Core Path

Core exports:

```text
NAMESPACE_PATH_MAX_DEPTH = 4

NamespacePathSegment {
    authority: CapabilityId,
    key: NamespaceKey,
}

resolve_namespace_path(
    actor,
    root_workspace,
    segments: &[NamespacePathSegment],
) -> NamespacePathResolution
```

`NamespacePathResolution` returns the root Workspace, terminal complete Entry
record, and resolved depth. The request depth must be in `1..=4`.

Resolution validates in this order:

1. authenticate an active Agent;
2. reject an empty or oversized path;
3. for each segment, require its Observe Capability on the current Workspace;
4. resolve the segment key in that Workspace;
5. for every non-terminal segment, require `Mount(target)` and advance to the
   active target Workspace;
6. reject any repeated Workspace as `NamespaceMountCycle`;
7. reserve one Event slot per segment;
8. append the ordered per-hop resolution Events;
9. return the terminal complete Entry record.

Authorization precedes lookup at every hop. A caller cannot probe keys in a
Workspace without authority for that exact Workspace. Full-path validation
precedes Event emission, so a late failure cannot leave a partial audit trail.

## Agent Call 50

`ResolveNamespacePath = 50` carries one or two segments without a userspace
pointer. Longer native paths remain available through the Core/facade API and
will gain user-memory transport in a later milestone.

| Register | Value |
| --- | --- |
| `r10` | root Workspace ID |
| `r11` | depth, `1` or `2` |
| `r12` | segment 1 Observe Capability |
| `r13` | segment 1 key |
| `r14` | segment 2 Observe Capability, zero at depth 1 |
| `r15` | segment 2 key, zero at depth 1 |
| `rbp` | zero |

The canonical reply uses the complete Namespace Entry layout shared by Calls
44 through 49. Its operation identity is 50 and its record is the terminal
Entry.

Packed Namespace object tag 6 identifies `Mount(ResourceId)`. Tag 7 remains
reserved. Zero identity words, unsupported depth, absent second authority, and
noncanonical depth-1 payloads fail ABI decoding.

## Bare-Metal Proof

The Resource Manager changes its first managed Resource to active Workspace 3
and keeps it live. Existing lifecycle operations still derive and revoke its
child Capability.

The native Namespace sequence becomes:

1. bind Entry 1 in root Workspace 1 at key `0x4e53_0001` to
   `Mount(Workspace 3)` with Capability 12;
2. bind Entry 2 in Workspace 3 at key `0x4e53_0002` to MemoryCell 2 with
   Capability 13;
3. resolve the two-hop path with independent Capabilities 12 and 13;
4. compare-and-rebind Entry 2 at revision 1 to Agent 8, producing revision 2;
5. compare-and-retire mount Entry 1 at revision 1.

The removed Resource-retirement Event is replaced by the additional path-hop
Event, preserving the global transcript size. The exact window is:

```text
event[184] resource_retired
event[185] namespace_entry_bound       # root mount
event[186] namespace_entry_bound       # child terminal
event[187] namespace_entry_resolved    # root mount hop
event[188] namespace_entry_resolved    # child terminal hop
event[189] namespace_entry_rebound     # child revision 2
event[190] namespace_entry_retired     # mount removed
event[191] resource_created
```

Required terminal evidence:

- Workspace 3 remains active and owned by Resource Manager;
- Entry 1 is absent;
- Entry 2 remains in Workspace 3 with revision 2 and object Agent 8;
- Namespace capacity is 2 and final occupancy is 1;
- Resource Manager executes 38 Calls and 76 address-space switches;
- live Event capacity remains 362;
- archive replay remains the exact Events `1..396` transcript;
- debug and release QEMU agree on markers, offsets, Capsule bytes, and digest.

The persistent Workspace changes the later Supervisor cleanup profile. Resource
3 and Capability 13 remain active, while revoked Capability 14 remains as audit
state. The Supervisor compacts transient Capabilities 26 and 27, revokes Memory
Capability 16 before capacity reuse, then retires Resource 4 after its
MemoryCell record and Capability reference leave the Stores. Revoked Region
Capability 17 remains inspectable.

| Verified artifact | Value |
| --- | --- |
| Resource / Capability / Namespace capacity | `8 / 28 / 2` |
| Resource Manager Calls / CR3 switches | `38 / 76` |
| Resource Manager Capsule | `3,789` bytes |
| Resource Manager SHA-256 | `24d6a22464c9b2cc27826c6b07a4655a5510968286eaff7c632732b408bdcc1a` |
| Admission Supervisor Calls / CR3 switches | `44 / 88` |
| Admission Supervisor Capsule | `4,115` bytes |
| Admission Supervisor SHA-256 | `f6c4efffe3c58689f8cb926399dc3fcb675e938d95bba463130495696f72f3f2` |
| Debug / Release transcript | Events `1..396`, `SUPERVISOR_HANDOFF_READY` |
| Release ELF occurrence | each Capsule `1`, each code body `1` |

## Verification Gates

- failing Core contracts before implementation;
- mount kind, retirement, direct cycle, transitive cycle, and rebind-cycle
  atomicity;
- empty, oversized, missing, non-mount, stale authority, late-hop authority,
  and Event-exhaustion path failures;
- facade and x86 Call 50 contracts;
- Workspace tests, Supervisor simulation, `no_std`, strict Clippy, and
  bare-metal compile;
- debug and release QEMU transcript validation;
- independent Resource Manager assembly and exact Release ELF occurrence.

## Deferred Work

- architecture-neutral user-memory transfer for three- and four-hop paths;
- Namespace path mutation and mount delegation receipts;
- durable signed Namespace snapshots and distributed generation proofs.
