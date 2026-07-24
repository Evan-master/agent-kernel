# State Signer Agent V16 Design

## Goal

V16 closes the signed durable archive transaction across the Agent Call
boundary:

```text
ring-3 State Signer
    prepare call
        -> Core authority preflight
        -> canonical manifest staged in private call-data
    sign through injected provider
    commit call
        -> snapshot and canonical decode
        -> Ed25519 policy verification
        -> ATA transaction
        -> one-shot Core release
```

The State Signer owns signing policy and key-provider access. Kernel code owns
authorization, Event selection, canonical preparation, storage I/O, readback,
and final state mutation.

V16 introduces no tracked private key and no kernel signing primitive.

## Layer Ownership

| Layer | V16 responsibility |
| :--- | :--- |
| `agent-kernel-core` | read-only durable preflight record and shared commit validation |
| `agent-kernel` | public preflight facade and verified final release |
| `agent-kernel-x86_64` | request staging, one-shot ATA preparation, Agent Call ABI, native handler |
| `agent-state-signer` | `no_std` signer policy and injected signing-provider boundary |
| native boot binary | wire the optional ATA session into Agent Call execution |

Private-key loading, TPM/HSM drivers, and production secret provisioning stay
outside the kernel.

## Core Preflight

The machine must reject authority and proposal failures before the first
storage write. Core therefore exposes a read-only durable preflight.

Inputs:

- actor Agent;
- root Rollback Capability;
- storage Checkpoint Capability;
- expected storage Resource;
- immutable Event archive proposal.

Checks:

1. actor has a launched Supervisor entry;
2. archive Capability belongs to actor and targets an active root Resource;
3. archive Capability grants `Rollback`;
4. proposal equals the current canonical Event prefix;
5. storage Resource is active;
6. storage Capability belongs to actor and grants `Checkpoint`.

The returned `DurableArchivePreflight` records the exact actor, authorities,
root, storage, and proposal. Preflight emits no Event because it performs no
state mutation.

Final commit repeats the same validation. The privileged handler runs
preflight, storage commit, and Core release without returning to ring 3 between
the storage and Core phases.

## Prepared Request

V16 reuses the 384-byte V15 request format:

```text
offset  size  field
0       8     magic = "AKDARQ15"
8       2     format version = 1
10      2     flags = 0
12      4     total length = 384
16      8     call-data generation
24      8     storage Checkpoint Capability
32      285   canonical durable manifest
317     64    Ed25519 signature
381     3     reserved = 0
```

During prepare, the kernel writes the complete record with a zero signature.
The State Signer may change only bytes `317..381`. Commit requires every other
byte to decode to the exact stored preparation.

The manifest binds:

- archive generation and exact Event prefix;
- actor and root Rollback authority;
- root and storage Resources;
- payload length and digest;
- State Signer identity and policy generation;
- rollback-evident anchor policy.

## ATA Session State

One initialized ATA session has three logical states:

| State | Accepted operation |
| :--- | :--- |
| ready | prepare |
| prepared | matching commit, retry after pre-I/O rejection, or trusted cancel |
| faulted | reinitialization only |

Preparation stores:

- Agent, Task, and Image identity;
- archive and storage authorities;
- call-data generation;
- proposal and canonical manifest;
- exact payload length retained in the session payload buffer.

A second prepare cannot replace live preparation.

Manifest mismatch, stale generation, wrong caller, and invalid signature cause
zero device writes and retain the preparation. Once the ATA transaction starts,
any transaction failure faults the session. Reinitialization must scan media
before later commits.

Successful transaction clears the preparation and advances the backend head.
The one-shot verified commit is then consumed by Core.

## State Signer Agent

`agent-state-signer` is a portable `no_std` user-space component. It owns no
concrete private-key format.

```rust
pub trait StateSignerProvider {
    type Error;

    fn signer_id(&self) -> DurableStateSignerId;
    fn sign_manifest(
        &mut self,
        manifest: &[u8; DURABLE_ARCHIVE_MANIFEST_BYTES],
    ) -> Result<DurableArchiveSignature, Self::Error>;
}
```

The Agent enforces an independent policy:

- expected root Resource;
- expected storage Resource;
- expected signer identity;
- expected signer policy generation;
- exact call-data generation;
- empty signature field before signing.

Software Ed25519, TPM, HSM, remote custody, and sealed-device providers can
implement the same interface. Test providers use deterministic development
keys only inside test targets.

## Agent Call ABI

V16 reserves two operation IDs:

| ID | Operation |
| ---: | :--- |
| `54` | `PrepareDurableArchive` |
| `55` | `CommitDurableArchiveFromMemory` |

### Prepare Registers

```text
r10 archive Rollback Capability
r11 storage Checkpoint Capability
r12 through Event sequence
r13 call-data generation
r14 r15 rbp = 0
```

The success reply returns generation, first sequence, through sequence, Event
count, manifest bytes, request bytes, and signer policy generation.

### Commit Registers

```text
r10 call-data generation
r11 r12 r13 r14 r15 rbp = 0
```

The privileged handler snapshots exactly 384 bytes while ring 3 is stopped.
The success reply returns the committed checkpoint range and digest.

The legacy `ArchiveEvents(40)` operation remains durability-gated.

## Failure Semantics

- Core preflight failure performs no call-data write and no device I/O.
- Request staging requires the active kernel CR3 and the Agent-owned physical
  call-data frame.
- A stale or altered request performs no device write.
- Signature verification completes before the first ATA write.
- Transaction failure faults the session and preserves live Core Events.
- Core release failure after a successful storage commit is fatal to the
  native handler; the durable head remains recoverable on reboot.
- A verified commit can authorize one Core mutation.

## Verification Gates

- Core tests cover valid preflight, authority rejection, stale proposals, and
  mutation-free failure.
- Request tests freeze the unsigned 384-byte record and public offsets.
- ATA tests cover prepare, signed commit, retryable pre-I/O rejection,
  conflicting preparation, and faulted-session behavior.
- State Signer tests cover policy enforcement and provider failures.
- A host integration test executes preflight, prepare, Agent signing, ATA
  commit, Core release, and cold recovery.
- Agent Call tests freeze IDs, register decoding, authentication, and replies.
- Workspace tests, strict Clippy, Supervisor replay, and bare target
  compilation pass.

## Exclusions

V16 excludes production key provisioning, a checked-in private key, TPM/HSM
drivers, a packaged native signer executable, complete object-graph replay,
NVMe, filesystems, and emulator power-loss execution.
