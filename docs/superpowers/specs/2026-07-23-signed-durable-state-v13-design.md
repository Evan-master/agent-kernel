# Signed Durable State V13 Design

## Goal

V13 makes Event Archive release conditional on a signed, crash-consistent
storage commit. An archived Event prefix remains live in kernel memory until a
trusted backend has flushed, read back, and verified one complete durable
capsule.

The protocol stays Agent-native. It uses Resources, Capabilities, checkpoint
generations, verifier policy, and immutable receipts. Filesystems, paths,
process credentials, and POSIX durability APIs do not define the kernel model.

## Layer Ownership

| Layer | Responsibility |
| :--- | :--- |
| `agent-kernel-core` | proposal identity, authority, generation binding, receipt validation, atomic Event release |
| `agent-kernel` | syscall facade without exposing verifier injection to Agents |
| `agent-kernel-hal` | bounded slot write, flush, readback, and commit-footer contract |
| `agent-kernel-x86_64` | canonical capsule parsing, Ed25519 verification, native storage adapter |
| `agent-supervisor` | signing orchestration and deterministic crash-recovery tests |

Model inference, private signing keys, recovery policy, and storage selection
remain outside kernel space. The kernel owns deterministic validation and the
point at which volatile Events become releasable.

## Threat Model

V13 covers power loss between writes; torn, truncated, reordered, duplicated,
or corrupted slot data; stale capsules; altered manifests; foreign receipts;
and receipt replay. Ed25519 private keys remain outside kernel memory. The
kernel selects a trusted HAL backend and accepts flush completion according to
that device contract.

A signed disk chain cannot detect restoration of an older, internally valid
disk image after a cold boot. Cross-boot rollback resistance requires TPM NV,
platform NVRAM, or a remote witness. V13 binds optional anchor generation and
digest evidence into every receipt. Profiles without an anchor advertise
`rollback-evident`, never `rollback-resistant`.

## Core Values

| Value | Invariant |
| :--- | :--- |
| `DurableStateSignerId` | domain-separated SHA-256 identity of one Ed25519 public key |
| `DurableArchiveManifest` | exact proposal, authority, storage Resource, signer policy, and payload length |
| `DurableArchiveSignature` | 64-byte Ed25519 signature over the canonical manifest |
| `DurableSlot` | slot `A` or `B`; callers never provide raw block offsets |
| `DurableArchiveReceipt` | slot, generation, manifest/readback digests, flush epoch, and anchor evidence |
| `DurableArchiveCommitProof` | verifier result consumed exactly once by Core commit |

Raw receipts remain untrusted. `KernelCore` calls a trusted
`DurableArchiveVerifier` supplied by the facade and creates the internal proof
inside the same commit operation. Agent callers cannot inject a verifier or
construct proof tokens.

### Frozen Value Contracts

- State Signer IDs are SHA-256 over
  `AGENT-KERNEL-DURABLE-STATE-SIGNER-V1\0 | public_key`. This domain remains
  distinct from the Agent Image signer domain even when both policies contain
  the same Ed25519 public key.
- Durable digests and signer IDs are 32 bytes. Ed25519 signatures are exactly
  64 bytes. Persisted lengths use `u32`; Event counts use `u16`; generations,
  sequence numbers, object IDs, policy generations, and flush epochs use
  `u64`.
- Odd archive generations target slot `A`; even generations target slot `B`.
  Generation zero has no slot.
- An unanchored profile carries generation zero and the zero digest. A trusted
  anchor carries the immediately preceding archive generation and digest.
  Trusted genesis is the explicit `(Trusted, 0, zero digest)` tuple.
- A recovery head labels an unanchored chain `rollback-evident`. It may label a
  trusted-anchor chain `rollback-resistant` only after machine verification of
  the manifest, receipt, and anchor evidence.

## Canonical Archive Payload

The payload is the exact byte preimage used by Event Archive SHA-256:

```text
domain | format_version | generation | previous_through_sequence
previous_digest | first_sequence | through_sequence | event_count
canonical_event[0..event_count]
```

Integers are little-endian. Optional fields use one presence byte. Enum values
use frozen numeric tags. Rust layout, padding, pointer width, and host
serialization libraries never enter the format.

The current digest encoder becomes a sink-based canonical encoder. One sink
feeds SHA-256; another writes into a caller-provided slice. Encoding fails
atomically on insufficient capacity. Re-hashing stored payload must equal the
proposal digest. One segment is limited to 64 Events. Its payload is limited to
`64 KiB - 512 bytes`, leaving fixed protocol space inside one 64 KiB slot.

## Signed Manifest

The signing message uses domain `AGENT-KERNEL-DURABLE-ARCHIVE\0`, a fixed
format version, and the canonical manifest bytes. The manifest binds archive
generation and range, previous/current archive digests, actor, archive
authority, root Resource, storage Resource, payload length and digest, State
Signer identity and policy generation, plus requested anchor evidence.

Unknown flags, nonzero reserved bytes, zero identities, inconsistent counts,
and unsupported versions fail before signature verification. State Signer
trust is separate from Agent Image trust. Reusing a key requires two explicit
policy records.

The V13 signing message is exactly 285 bytes:

```text
domain[29] | version:u16 | flags:u16 | reserved[4]
generation:u64 | first:u64 | through:u64 | event_count:u16 | reserved[6]
previous_digest[32] | archive_digest[32]
actor:u64 | archive_authority:u64 | root:u64 | storage:u64
payload_length:u32 | reserved[4] | payload_digest[32]
state_signer_id[32] | signer_policy_generation:u64
anchor_generation:u64 | anchor_digest[32]
```

Flag bit zero selects the trusted-anchor profile. All other flag bits and all
reserved bytes are zero in V13. Verification first resolves exactly one active
State Signer record, then checks key identity, root scope, current policy
generation, and the strict Ed25519 signature over all 285 bytes.

## Two-Slot Transaction

Each storage Resource owns equal fixed slots `A` and `B`. Generation parity
selects the inactive target. The backend verifies that target does not contain
the active committed generation.

Each slot is exactly 64 KiB: a 64-byte prepared header, a 65,408-byte body, and
a 64-byte commit footer. The HAL accepts semantic regions only; callers never
submit device offsets. Header and footer writes require exact lengths. Body
writes are non-empty and bounded by the body region. Every flush returns a
monotonic nonzero epoch, and readback returns the complete fixed slot.

1. Write a `Prepared` header to the inactive slot and flush.
2. Write canonical Event payload and signed manifest, then flush.
3. Read back and verify bounds, digests, signature, policy, proposal, and anchor.
4. Write a `Committed` footer containing generation and manifest digest.
5. Flush and read back the footer.
6. Return one `DurableArchiveReceipt` to the kernel facade.
7. Revalidate proposal and receipt, release the Event prefix atomically, and
   advance the in-memory archive chain head.

The footer is written last. Header state alone never marks a slot recoverable.
Any failure before Core commit leaves the Event prefix unchanged.

The deterministic Supervisor backend maintains separate volatile and durable
bytes plus phase metadata for both slots. A flush copies one complete slot and
its metadata into the durable image and advances a global epoch. Injected power
loss occurs before a selected write, flush, or readback operation, discards all
unflushed state, and leaves the prior committed generation active. Tests cover
all eight operation boundaries in one complete transaction.

## Recovery

Recovery accepts a slot only when header and footer both declare `Committed`,
generation and manifest digest agree, lengths fit V13 bounds, payload and
archive digests recompute, signature and signer policy validate, and the
previous digest links to an accepted predecessor or trusted anchor.

The highest valid connected generation wins. Equal-generation disagreement,
disconnected valid heads, anchor mismatch, or generation overflow stops
automatic recovery with a precise failure reason.

## Authority And Atomicity

Prepare requires an existing Event prefix. Commit requires a launched
Supervisor, root-scoped `Rollback` authority, `Checkpoint` authority for the
storage Resource, an active State Signer scoped to the same root, and a receipt
matching the current proposal and storage Resource.

All checks precede mutation. Failures preserve Event storage, archive head,
signer policy, receipt replay state, and slot selection. The signed manifest,
commit footer, receipt, and chain head provide audit evidence. A second Event
would introduce recursive digest dependency, so this mutation has no extra
Event record.

## Verification Gates

- Canonical payload bytes hash to the existing archive digest.
- Signatures fail after any payload, manifest, signer, policy, or authority edit.
- Crash injection at every write/flush recovers the old or fully committed slot.
- Missing, stale, foreign, duplicate, and replayed receipts fail atomically.
- Event prefixes release only after verified readback and footer flush.
- Recovery rejects split-brain heads and anchor rollback.
- Unanchored recovery is labeled rollback-evident.
- Format and verifier compile for `x86_64-unknown-none`.

A later native block profile adds debug/release QEMU power-loss matrices without
changing Core or capsule contracts.

## Exclusions

V13 excludes filesystems, path APIs, private keys in kernel memory, wear
leveling, RAID, replication, distributed consensus, allocator-backed kernel
buffers, cold-boot rollback-resistance claims without an anchor, and complete
KernelCore object-graph replay from Events.
