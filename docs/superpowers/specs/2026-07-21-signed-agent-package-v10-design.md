# Signed Agent Package V10 Design

Status: Approved for implementation on 2026-07-21

## Objective

Extend the segmented Agent Package with an authenticated signer identity and
an immutable kernel boot trust policy. V10 makes signature verification a
mandatory loader gate for signed packages before private frames are allocated,
relocated, mapped, or entered at ring 3.

Package v3 retains the V9 code, rodata, and bounded `ABS64` contracts. Capsule
v1 and Package v2 remain parseable as explicitly digest-pinned legacy images.
The Resource Manager migrates to v3 and becomes the first signed native Agent.

## Package v3 Header

Package v3 keeps the eight-byte `AGNTIMG\0` magic. All integers are
little-endian. The canonical header is 88 bytes.

| Offset | Bytes | Field | Canonical value |
| ---: | ---: | :--- | :--- |
| `0` | 8 | magic | `AGNTIMG\0` |
| `8` | 2 | format version | `3` |
| `10` | 2 | architecture | `1` / x86_64 |
| `12` | 2 | image kind | `1..4` |
| `14` | 2 | package flags | `1` / signed |
| `16` | 2 | ABI version | nonzero |
| `18` | 2 | entry version | nonzero |
| `20` | 2 | entry segment | `0` / code |
| `22` | 2 | reserved | `0` |
| `24` | 4 | entry offset | inside code |
| `28` | 2 | segment count | `2` |
| `30` | 2 | relocation count | `0..64` |
| `32` | 4 | segment table offset | `88` |
| `36` | 4 | relocation table offset | `136` |
| `40` | 4 | signature offset | exact end of rodata |
| `44` | 4 | package length | signature offset + `64` |
| `48` | 32 | signer ID | derived from the trusted public key |
| `80` | 2 | signature algorithm | `1` / Ed25519 |
| `82` | 2 | signature length | `64` |
| `84` | 4 | reserved | `0` |

Two canonical 24-byte segment descriptors follow the header. Their fields and
limits match Package v2:

| Index | Kind | Flags | Alignment | File bytes | Memory bytes |
| ---: | :--- | :--- | ---: | :--- | :--- |
| `0` | code | `R + X` | `4096` | `1..65,536` | exact file length |
| `1` | rodata | `R` | `4096` | `1..65,536` | exact file length |

The relocation table follows at offset `136`. Packed code and rodata payloads
follow the relocation records. The 64-byte signature is the final package
field. Gaps, trailing bytes, unknown algorithms, and nonzero reserved fields
are rejected.

## Signature Contract

The Ed25519 message is the exact package prefix
`package[0..signature_offset]`. This binds:

- architecture, kind, ABI, entry, and flags;
- signer identity and signature algorithm;
- segment descriptors and relocation records;
- every code and rodata byte.

The complete package, including the signature, remains bound to the
`AgentImageRecord` SHA-256 digest. Verification uses
`VerifyingKey::verify_strict` to reject weak-key and noncanonical signature
cases.

Signer ID derivation is domain-separated:

~~~text
SHA-256("AGENT_KERNEL_ED25519_SIGNER_V1\0" || ed25519_public_key)
~~~

The public key is never supplied by the package. The signer ID selects one
kernel-owned trust-policy entry.

## Boot Trust Policy

`AgentImageTrustPolicy<N>` is fixed-capacity and heap-free. Each immutable
entry contains:

| Field | Policy meaning |
| :--- | :--- |
| signer ID | exact domain-separated public-key identity |
| public key | 32-byte Ed25519 verification key |
| image-kind scope | bounded mask for Worker, Verifier, Fault Handler, Supervisor |
| ABI interval | inclusive minimum and maximum ABI versions |
| status | `Active` or `Revoked` |

Authorization requires exactly one matching entry. Missing, duplicate,
revoked, key-ID-mismatched, out-of-scope, and out-of-range entries fail before
signature arithmetic succeeds.

V10 keeps the policy immutable during one boot. Runtime signer enrollment,
revocation Events, persistent trust state, and key rotation transactions remain
future kernel-state milestones.

## Loader Boundary

`VerifiedAgentImage::verify` accepts digest-pinned Capsule v1 and Package v2
images. It rejects Package v3 with `SignatureVerificationRequired`.

`VerifiedAgentImage::verify_signed` requires:

1. canonical Package v3 parsing;
2. kernel record status, metadata, and complete-package digest agreement;
3. one active Trust Policy signer matching kind and ABI scope;
4. signer ID derivation agreement;
5. strict Ed25519 verification over the canonical prefix.

Only the resulting signed `VerifiedAgentImage` may enter the existing frame
allocation, relocation, page-table installation, and ring-3 execution path.
The value carries its verified signer ID as loader evidence.

## Native Proof

The Resource Manager becomes a signed Package v3 while retaining its five code
pages, one rodata page, one relocation, and 43-call native transcript. The
repository tracks only its public trust anchor and detached package signature.
No private signing key is committed.

Successful QEMU completion must emit:

- `AGENT_KERNEL_NATIVE_SIGNED_PACKAGE_OK`;
- `AGENT_KERNEL_NATIVE_TRUSTED_SIGNER_OK`;
- the existing segmented-package, rodata-NX, relocation, Event replay, and
  Supervisor handoff markers.

These markers are emitted only after signed loader evidence, package mapping
evidence, and Resource Manager execution evidence agree.

## Verification Gates

- V3 parser tests lock every header offset, signature boundary, and canonical
  segment/relocation rule.
- Signature tests prove valid authorization and reject message, signature,
  signer-ID, public-key, algorithm, digest, and metadata tampering.
- Trust-policy tests reject missing, duplicate, revoked, kind-scoped, and
  ABI-scoped signers.
- Legacy tests prove V1/V2 parsing and digest-pinned loading remain explicit.
- The Resource Manager audit reproduces descriptors, payloads, signer ID,
  strict signature verification, SHA-256, assembly bytes, and Release ELF
  uniqueness.
- Focused tests, Workspace tests, Supervisor, five `no_std` targets, strict
  Clippy, debug and Release QEMU pass.

## Deferred Work

- runtime trust-policy mutations with capability authorization and Events;
- persistent signed trust state and transactional key rotation;
- signing the remaining boot inventory;
- multiple read-only data segments and writable initialized data;
- richer relocations, public package-builder CLI, demand paging, and SMP TLB
  synchronization.
