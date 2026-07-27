# QEMU ATA Power-Loss V26 Design

## Goal

V26 activates the dormant native durable-boot path against real QEMU devices:

```text
ring-3 State Signer
        |
        +-- Agent Call 54: prepare Events 1..64
        +-- Agent Call 56: sign through TPM CRB
        +-- Agent Call 55: commit through ATA PIO
                                  |
                                  +-- FLUSH CACHE EXT
                                  +-- verified Core release
                                  +-- abrupt QEMU termination

same raw ATA image
        |
        +-- cold boot scan
        +-- signature and chain verification
        +-- Core recovery import
        +-- first new Event sequence = 65
```

The proof uses the existing V13 through V20 protocol and hardware adapters.
V26 adds machine activation, an executable State Signer flow, abrupt-power
orchestration, and recovery-aware Event handling.

## Profiles

The normal `bare-metal` profile stays disk-free and preserves the exact V25
QEMU transcript.

The `qemu-durable-proof` feature accepts one generated public profile with two
roles:

| Role | ATA | TPM | State Signer | Terminal condition |
| --- | --- | --- | --- | --- |
| writer | enabled | enabled | enabled | commit marker, then host kill |
| recovery | enabled | disabled | disabled | normal debug exit |

Both roles use the same kernel source, durable public key, storage Resource ID,
ATA geometry, and policy generation. The writer and recovery images are
separate build artifacts so the recovery run cannot issue another commit.

## Device Topology

QEMU exposes:

- the BIOS boot image as legacy ATA primary master;
- one dedicated raw durable image as legacy ATA primary slave;
- a TPM 2.0 CRB device backed by `swtpm` during the writer run;
- the existing PCI serial device and interrupt route.

The native storage profile uses command base `0x1f0`, control base `0x3f6`,
slave selection, LBA 0, 512-byte sectors, and the existing bounded poll budget.
The durable image contains no partition table or boot payload.

## External TPM State

The proof harness creates a fresh `swtpm` state directory with mode `0700`.
It provisions one unrestricted P-256 signing key at persistent handle
`0x81010001`.

Authorization binds:

- SHA-256 PCR 23;
- the expected all-zero PCR composite for a fresh emulator boot;
- `TPM2_CC_Sign`.

The harness exports only the Name, compressed public point, PCR policy,
signer ID, and policy generation into the generated build profile. TPM private
state and the ephemeral Agent-image signing key remain outside the repository
and are deleted with the proof workspace.

## Recovery-Aware Supervisor

The admission Supervisor currently requests absolute Event sequence 64. A
recovered Core starts at the durable head plus one, so V26 supplies the current
boot's first Event sequence through the read-only signal page.

The Supervisor requests:

```text
through = boot_first_sequence + 63
```

It requires a 64-Event response and validates nonzero digest words. Kernel-side
evidence recomputes the complete proposal and checks:

- generation against the prior checkpoint;
- previous digest against the prior checkpoint;
- dense Event sequences from the boot base;
- unchanged durable checkpoint and receipt for retained snapshots.

Genesis remains `1..64`. Recovery from the V26 writer becomes `65..128`.

## Delayed Storage Binding

ATA identity and slot recovery run before Core boot. The Device Resource is
registered only after the existing Runtime Admission flow completes.

This order preserves all V25 object identities and fixed admission evidence.
Writer and recovery runs produce the same storage Resource identity because
durable recovery imports only the Event head and next sequence, not a mutable
object graph.

The binding grants the bootstrap Agent `Checkpoint | Delegate`. A dedicated
State Signer receives:

- `Rollback` on the root Resource;
- `Checkpoint` on the ATA Device Resource;
- its delegated task Capability.

No raw port, ATA, TPM MMIO, or arbitrary TPM command Capability reaches ring 3.

## State Signer Execution

The writer embeds a generated signed Package v3 built from the repository's
auditable State Signer entry and kernel TPM provider.

The native flow:

1. trusts the package's ephemeral public image key for State Signer images;
2. reuses reclaimed `AgentId(11)` and launches one
   `AgentEntryKind::StateSigner`;
3. reconstructs its two-segment address space from reclaimed zeroed frames;
4. executes Calls 54, 56, and 55 in ring 3;
5. verifies one completed task, one released 64-Event prefix, and an empty
   runtime;
6. performs TLB shootdown and returns every address-space frame;
7. emits `AGENT_KERNEL_QEMU_DURABLE_COMMIT_OK`.

The host kills QEMU immediately after observing this marker. Guest shutdown and
the normal debug-exit path are not used for the writer.

## Build-Time Public Profile

The x86 crate build script always creates one `OUT_DIR` profile module.

- Without complete V26 environment input it emits a disabled profile.
- With complete input it validates lengths, hexadecimal encoding, roles,
  nonzero IDs, and package path before emitting public constants.
- Cargo rebuild directives cover every environment field.

Feature-enabled builds without V26 environment input therefore remain
self-contained through the disabled profile. A requested writer or recovery
profile fails closed when its input is incomplete.

## Disk Inspection

The disk inspector reads both fixed slots and validates:

- prepared header and commit footer agreement;
- canonical manifest fields;
- payload bounds and SHA-256 digest;
- ECDSA P-256 signature with the provisioned public key;
- slot generation and previous-head linkage;
- selection of the newest valid committed slot.

The writer proof requires generation 1, through sequence 64, and a committed
head. The recovery run must leave the same head byte-for-byte unchanged.

## Failure Semantics

- ATA or TPM activation failure halts before Agent execution.
- A public profile mismatch halts before storage mutation.
- State Signer package, identity, Capability, or policy mismatch halts before
  Call 55.
- TPM policy or signature failure leaves both ATA slots without a new committed
  head.
- ATA write, readback, or flush failure preserves live Core Events.
- A recovery verification failure leaves a virgin Core unchanged.
- Missing commit markers, graceful writer exit, changed recovery disk bytes,
  or a noncontiguous Event sequence fail the host proof.

## Verification

V26 adds:

- recovery-relative Supervisor contract tests;
- generated-profile fail-closed tests;
- State Signer setup and authority tests;
- durable disk inspector fixtures and rejection tests;
- writer and recovery QEMU transcript validators;
- an abrupt-power two-boot integration script;
- default debug and release V25 regression runs;
- workspace tests, strict Clippy, formatting, Supervisor replay, package
  audits, bare-target checks, and shell/Ruby syntax gates.

## Validated Evidence

| Check | Debug | Release |
| --- | --- | --- |
| writer termination | host `SIGKILL` | host `SIGKILL` |
| durable head | generation 1, Events `1..64` | generation 1, Events `1..64` |
| recovery TPM | absent | absent |
| recovery history | Events `65..516` | Events `65..516` |
| PCI serial byte | `0x50` | `0x50` |
| media mutation during recovery | none | none |

Both profiles emit `AGENT_KERNEL_QEMU_DURABLE_POWER_LOSS_OK` only after the
offline inspector validates the signed slot and the pre/post recovery disk
hashes match.

## Exclusions

V26 does not add ATA DMA, AHCI, NVMe, IOMMU domains, MSI/MSI-X, encrypted
durable payloads, partition discovery, multi-device failover, or production
key provisioning.
