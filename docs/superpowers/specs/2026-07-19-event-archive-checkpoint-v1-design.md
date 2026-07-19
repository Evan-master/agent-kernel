# Event Archive Checkpoint V1 Design

Status: Implemented and validated

## Objective

Recover bounded Event Log capacity without resetting Event sequence numbers or
discarding the ability to authenticate retired history. A launched Supervisor
can hand off a dense Event prefix to an external archive, then commit a
kernel-owned checkpoint whose SHA-256 digest chains that segment to every
previous archive checkpoint.

This milestone keeps the AgentOS object model allocator-free and `no_std`.
Durable storage media remain outside Core. The x86 reference profile proves the
handoff with a bounded architecture-owned archive buffer.

## Ownership Boundaries

- `agent-kernel-core` owns canonical Event encoding, digest construction,
  proposal validation, root-scope authorization, dense prefix removal, the
  latest checkpoint, and monotonic Event sequencing.
- `agent-kernel` exposes read-only proposal preparation and authorized commit
  methods without exposing mutable Core state.
- `agent-kernel-x86_64` owns Agent Call 40, the architecture archive buffer,
  complete pre-commit Event capture, reply encoding, and bare-metal evidence.
- `agent-supervisor` may format checkpoints and persist exported Event segments;
  persistence policy and media never enter Core.

## Public Records

`EventArchiveDigest` contains exactly 32 SHA-256 bytes and exposes a canonical
four-word little-endian representation for the x86 ABI.

`EventArchiveProposal` contains:

- archive generation;
- first and final Event sequence numbers;
- Event count;
- previous archive digest;
- resulting chained digest.

`EventArchiveCheckpoint` contains the accepted proposal plus the Supervisor,
root Resource, and Rollback Capability that authorized the commit.

Core retains one latest checkpoint. Older checkpoints and their Event segments
are external archive material. Every new digest commits the previous digest,
so replacing or reordering any historical segment changes the current chain
head.

## Two-Phase Protocol

1. `prepare_event_archive(through)` selects the dense live prefix ending at the
   requested sequence and returns a proposal without mutation.
2. The Supervisor or architecture adapter copies that exact Event segment to
   its archive target.
3. `commit_event_archive(actor, authority, proposal)` authenticates the caller,
   recomputes the proposal from current state, and compares the complete value.
4. On equality, Core removes the prefix in stable order, clears vacated slots,
   stores the new checkpoint, and leaves `next_sequence` unchanged.

A stale proposal, a changed Event prefix, an invalid authority, or an unknown
terminal sequence fails before mutation.

## Canonical Digest Format

The digest uses `sha2` with default features disabled. This dependency is
allocator-free, deterministic, `no_std`, already used by the x86 image layer,
and avoids a kernel-owned cryptographic implementation.

The SHA-256 input is:

```text
domain = "AGENT-KERNEL-EVENT-ARCHIVE\0"
format-version = 1
generation
previous-through-sequence
previous-digest
first-sequence
through-sequence
event-count
canonical Event 1
...
canonical Event N
```

Integers use fixed-width little-endian encoding. Optional values use one byte
for presence followed by their value when present. Enum tags have explicit
archive-format values. `NamespaceObject` includes its variant and typed ID.
Structured payloads encode each member in declaration order. Agent Image
digests contribute all 32 bytes. Every field in `Event` participates; Rust
layout, padding, enum discriminants, and pointer values never participate.

Segments must be non-empty and strictly contiguous. The first segment begins
at sequence 1. Every later segment begins one sequence after the retained
checkpoint high-water.

## Authorization

Commit requires all of the following:

- the actor has a launched `Supervisor` Agent Entry;
- the Capability is active, owned by the actor, and task-unscoped;
- the Capability authorizes `Operation::Rollback`;
- the Capability Resource is active and has no parent.

The root Resource rule reflects the fact that an Event prefix can contain
mutations from multiple descendant Resource domains.

## Audit Semantics

Archive commit updates a dedicated checkpoint instead of appending an Event to
the log being retired. This avoids recursive self-inclusion and permits commit
while the Event Log is full. The checkpoint records actor, authority, root
Resource, range, count, predecessor digest, and current digest. This dedicated
record is the audit consequence of the mutation.

## Agent Call 40

The native ABI adds:

```text
ArchiveEvents = 40
```

Request payload:

| Register | Value |
| --- | --- |
| `r10` | root Rollback Capability ID |
| `r11` | final Event sequence in the live prefix |
| `r12-r15`, `rbp` | zero |

Successful reply payload:

| Register | Value |
| --- | --- |
| `r10` | first archived Event sequence |
| `r11` | final archived Event sequence |
| `r12` | archived Event count |
| `r13-r15`, `rbp` | four little-endian digest words |

The handler snapshots every selected Event into a fixed local array, validates
architecture archive capacity, invokes the public facade, appends the snapshot
to the architecture archive only after Core commit succeeds, verifies the
checkpoint and retained suffix, then returns the canonical reply.

## X86 Full-Log Proof

The reference x86 profile reduces live Event capacity from 378 to 357. The
resident Admission Supervisor invokes Agent Call 40 after Event 357, when the
live log has no free slot, and archives Events 1 through 64.

Required terminal evidence:

- live occupancy is 357 of 357 before commit;
- the architecture archive contains exactly Events 1 through 64;
- the live log begins at Event 65 immediately after commit;
- final live occupancy is 314 while `next_sequence` reaches 379;
- archived and live iterators together reproduce Events 1 through 378 exactly;
- the Supervisor executes 34 Agent Calls and 68 address-space switches;
- Core and architecture checkpoints agree on range, count, actor, authority,
  root Resource, predecessor digest, and resulting digest;
- Debug and release QEMU produce the same digest and exact Event transcript.

### Validated Reference Values

- x86 live Event capacity: 357;
- archive generation and range: generation 1, Events 1 through 64, count 64;
- previous digest: 32 zero bytes;
- archive digest:
  `bcca87f797c97d77eea510de12bd94142e469566817d2758b692287b0edabf67`;
- little-endian ABI digest words: `0x777dc997f787cabc`,
  `0x1494bd12de10a5ee`, `0x58277d816695462e`, and
  `0x67bfda0e7b2892b6`;
- final live range and occupancy: Events 65 through 378, count 314, with
  `next_sequence` equal to 379;
- Admission Supervisor: 34 Agent Calls and 68 address-space switches;
- Supervisor return offsets: 44, 82, 169, 247, 361, 400, 495, 609, 648,
  766, 889, 976, 1054, 1168, 1207, 1302, 1416, 1455, 1573, 1693, 1813,
  1933, 2053, 2173, 2291, 2412, 2530, 2649, 2767, 2888, 3009, 3127,
  3267, and 3296;
- Supervisor code bytes: 3298;
- complete Supervisor Capsule bytes: 3330;
- Capsule SHA-256:
  `4f332e0bc22b8039b822ea4ab0a1f6600dc83a132a7267368e2a0bbde210e68c`;
- complete Capsule occurrences in the release ELF: 1;
- host Supervisor Events 1 through 64 digest:
  `6c3d502efb373196813fd512704a931e41bb5351834ee884581dce3d97965615`;
- debug and release QEMU each emit exactly 378 ordered Event lines and every
  required archive marker exactly once.

## Failure Rules

- missing launched Supervisor: existing Agent Entry errors;
- non-Supervisor caller: `AgentEntryKindMismatch`;
- empty log or unknown terminal sequence: `EventArchiveSequenceNotFound`;
- non-root authority Resource: `EventArchiveAuthorityScopeMismatch`;
- stale, altered, or foreign proposal: `EventArchiveProposalMismatch`;
- revoked, foreign, task-scoped, or attenuated authority: existing Capability
  errors;
- architecture archive capacity failure: fail closed before Core mutation.

## Deferred Work

- durable storage drivers and crash-consistent archive media;
- signed archive receipts and remote transparency logs;
- multiple independent archive consumers;
- selective Event query Agent Calls and zero-copy read-only mappings;
- checkpoint restoration and full state replay from archived Events.
