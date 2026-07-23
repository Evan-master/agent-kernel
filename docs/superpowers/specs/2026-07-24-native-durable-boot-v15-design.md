# Native Durable Boot V15 Design

## Goal

V15 turns the V14 ATA backend into a boot-owned durable archive service. A
verified disk head can seed Core before the first Event of a new boot, and a
signed user-space request can drive the existing V13 transaction without
placing private signing material in kernel memory.

The milestone preserves three boundaries:

- Core accepts recovery only through a one-shot machine verifier.
- The x86 machine layer owns ATA identity, slot recovery, and commit I/O.
- A Supervisor supplies a canonical manifest and Ed25519 signature through a
  bounded call-data record.

V15 does not add a State Signer Agent. The native Agent Call operation that
coordinates prepare, external signing, and final commit remains a follow-up.

## Layer Ownership

| Layer | V15 responsibility |
| :--- | :--- |
| `agent-kernel-core` | one-time recovered-head import and Event sequence continuation |
| `agent-kernel` | trusted recovery facade; no verifier injection through Agent syscalls |
| `agent-kernel-boot` | ordinary and recovered boot constructors |
| `agent-kernel-x86_64` | signed request decoding, ATA boot session, recovery proof, commit orchestration |
| `agent-supervisor` | future private-key owner and request producer |

The durable capsule format, V13 manifest, V14 sector layout, and HAL remain
unchanged.

## Core Recovery Handoff

`DurableRecoveredHead` carries structurally valid manifest and receipt values.
It does not carry an unforgeable proof of disk reads or signature verification.
V15 adds a separate `DurableArchiveRecoveryVerifier` boundary.

The machine layer returns `VerifiedDurableArchiveRecovery` only after:

1. both physical slots have been read;
2. capsule framing, payload digest, footer, and receipt fields have passed;
3. the State Signer policy and Ed25519 signature have passed;
4. the deterministic chain selector has chosen one head.

The value is one-shot. Core constructs the exact recovery request, invokes the
verifier, and imports the head only when the verifier accepts it.

Core recovery requires a virgin Event state:

- no live Events;
- no current archive checkpoint;
- no durable receipt;
- next Event sequence equal to one.

The recovered manifest is converted to the previous
`EventArchiveCheckpoint`. The durable receipt becomes the current replay guard.
The next Event sequence becomes `through_sequence + 1`. Sequence exhaustion
fails before mutation.

This import emits no Event. It restores previously authenticated history before
the current boot exists, so recording a new Event would create a circular
history dependency.

## Recovered Boot

`BootedKernel::boot_recovered` creates an empty facade, imports the verified
head, then runs the existing deterministic bootstrap sequence. The first new
bootstrap Event therefore follows the recovered archive sequence directly.

Ordinary `BootedKernel::boot` remains the genesis path and starts at Event
sequence one.

V15 restores the archive chain head and sequence clock. Complete object-graph
replay remains outside this milestone. Platform setup must recreate stable
Resource and Agent identities before accepting new work.

## Signed Request Record

One signed durable request occupies exactly 384 bytes in the Agent call-data
page:

```text
offset  size  field
0       8     magic = "AKDARQ15"
8       2     format version = 1
10      2     flags = 0
12      4     total length = 384
16      8     call-data generation
24      8     storage Checkpoint Capability
32      285   canonical V13 manifest
317     64    Ed25519 signature
381     3     reserved = 0
```

The decoder requires exact magic, version, length, generation, nonzero storage
authority, canonical manifest bytes, and zero reserved bytes. It performs no
memory access, authorization, cryptography, or I/O.

The generation is authenticated by the future Agent Call register request and
prevents a stale page from being interpreted as a new submission.

## Native ATA Boot Session

`NativeAtaDurableConfig` binds:

- validated ATA task-file configuration;
- expected root Resource;
- expected storage Resource;
- aligned base LBA;
- one active State Signer record and policy generation.

Initialization issues `IDENTIFY DEVICE`, validates the reserved two-slot range,
constructs the V14 backend, and scans both slots.

| Slot result | Boot result |
| :--- | :--- |
| both empty or only uncommitted data | bind `Genesis` |
| one connected verified head | bind `Recovered(generation)` and expose a one-shot recovery proof |
| corrupt, split, disconnected, revoked, or exhausted state | stop initialization |

Genesis is accepted only through the precise V13
`NoCommittedSlot` selection result. Transport, capsule, trust, and chain errors
remain distinct.

The session borrows three fixed buffers:

- 64 KiB ATA staging;
- 64 KiB capsule scratch;
- at most `MAX_DURABLE_ARCHIVE_PAYLOAD_BYTES` payload bytes.

No heap, global mutable buffer, filesystem, partition parser, or private key is
introduced.

## Commit Orchestration

The session accepts:

- the current Core proposal and exact Event prefix;
- actor, archive authority, and root identity;
- one decoded signed request.

It canonicalizes the Event prefix into the payload buffer and requires the
request manifest to match:

- proposal generation, range, count, previous digest, and archive digest;
- actor and root;
- archive authority and configured storage Resource;
- exact payload length and payload digest.

The configured State Signer policy verifies the signature. V13 then performs
the eight-operation transaction and returns
`VerifiedDurableArchiveCommit`. Core still owns final Event release through
`commit_verified_event_archive`.

The session never signs. Private key custody belongs to a user-space State
Signer Agent or an external signing adapter.

## Boot Configuration

The bare binary exposes an explicit durable storage profile:

- `Disabled` preserves the current disk-free boot path;
- `Ata` carries a complete validated `NativeAtaDurableConfig`.

An enabled profile must initialize storage before constructing the booted
kernel. A recovered session uses `boot_recovered`; a genesis session uses the
ordinary boot constructor.

The first checked-in profile remains `Disabled` until a dedicated ATA image and
State Signer Agent are attached. Both branches compile for
`x86_64-unknown-none`; host contract tests exercise the enabled branch through
a sector-backed device double.

## Failure Semantics

- Recovery verification failure leaves a virgin Core unchanged.
- A second recovery import is rejected before invoking the verifier.
- Sequence overflow leaves Core unchanged.
- Request decode failure performs no signature check or I/O.
- Manifest mismatch performs no device write.
- ATA initialization and recovery errors expose their source boundary.
- A failed commit preserves live Events; the final Core release remains gated
  by the one-shot verified commit.
- A consumed recovery or commit verifier cannot authorize a second mutation.

## Verification Gates

- Core tests prove one-shot recovery, sequence continuation, atomic rejection,
  and ordinary genesis behavior.
- Boot tests prove the first recovered bootstrap Event is contiguous with the
  persisted head.
- Request tests freeze all 384 bytes and reject stale, malformed, and
  noncanonical records.
- ATA boot-session tests cover genesis, recovered head binding, corrupt media,
  policy rejection, and full signed commit.
- Commit tests prove manifest mismatch causes zero block writes.
- Workspace tests, strict Clippy, formatting, Supervisor replay, and bare
  `x86_64-unknown-none` compilation pass.

## Exclusions

V15 excludes private-key custody, a native State Signer Agent, final Agent Call
prepare/commit operations, complete object-graph replay, TPM/NVRAM anchors,
partition discovery, AHCI, NVMe, DMA, filesystems, and emulator power-loss
execution.

