# Hardware State Signer Agility V18 Design

## Goal

V18 makes the durable archive signature contract usable by hardware-backed
providers without weakening the existing Ed25519 chain:

```text
DurableStatePublicKey
        -> algorithm-bound signer ID
        -> manifest v1 or v2
        -> State Signer policy
        -> strict machine verifier
```

Legacy Ed25519 archives retain their exact 285-byte manifest and signer-ID
encoding. New algorithm-bound manifests support Ed25519 and ECDSA P-256 with
SHA-256. The latter matches the signing primitive broadly exposed by TPM 2.0
and hardware security modules.

## Core Types

Core adds:

- `DurableSignatureAlgorithm::Ed25519`, stable value `1`;
- `DurableSignatureAlgorithm::EcdsaP256Sha256`, stable value `2`;
- `DurableStatePublicKey::Ed25519([u8; 32])`;
- `DurableStatePublicKey::EcdsaP256([u8; 33])`;
- `DurableArchiveManifestVersion::LegacyEd25519`, stable value `1`;
- `DurableArchiveManifestVersion::AlgorithmBound`, stable value `2`.

P-256 keys use canonical compressed SEC1 encoding. Prefixes other than `0x02`
or `0x03` fail safe construction. Native startup rejects malformed encodings,
and the machine verifier parses the complete curve point before signature
verification. Signer records bind one algorithm, public key, root Resource,
status, and policy generation.

The legacy Ed25519 signer ID remains:

```text
SHA-256("AGENT-KERNEL-DURABLE-STATE-SIGNER-V1\0" || public_key)
```

Algorithm-bound IDs use:

```text
SHA-256(
  "AGENT-KERNEL-DURABLE-STATE-SIGNER-V2\0" ||
  algorithm_u16_le ||
  canonical_public_key
)
```

Ed25519 records created through the legacy constructor retain their V1 ID.
P-256 records always use the algorithm-bound derivation.

## Manifest Encoding

The manifest remains exactly 285 bytes. V18 consumes two bytes from the
existing four-byte reserved field:

| Offset | Bytes | V1 | V2 |
| ---: | ---: | :--- | :--- |
| `29` | `2` | version `1` | version `2` |
| `31` | `2` | anchor flags | anchor flags |
| `33` | `2` | zero | signature algorithm |
| `35` | `2` | zero | zero |

V1 implicitly means Ed25519 and preserves every existing byte. V2 explicitly
binds the algorithm. Both formats retain the same field offsets after byte
`37`, the same 64-byte signature slot, and the same ATA capsule layout.

The old manifest constructor emits V1 Ed25519. The algorithm-bound constructor
emits V2 and requires a supported algorithm.

## Verification

The machine trust policy performs checks in this order:

1. exactly one signer ID matches;
2. the signer is active;
3. signer ID agrees with the stored key representation;
4. signer root and policy generation match;
5. manifest algorithm agrees with the signer key;
6. public-key parsing succeeds;
7. strict signature verification succeeds.

Ed25519 keeps `verify_strict`. ECDSA P-256 verifies SHA-256 over the canonical
manifest through RustCrypto `p256`. Signatures use fixed-width IEEE P1363
`r || s` encoding and must be low-S to keep one canonical representation.

## State Signer Boundary

`StateSignerProvider` declares its signature algorithm in addition to its
signer ID. `StateSignerPolicy` binds the same algorithm. The Agent rejects any
provider, policy, or manifest disagreement before invoking the provider.

The native provider function keeps its fixed three-argument ABI and 64-byte
output. Immutable package policy gains one signature-algorithm value. The
native entry validates:

- manifest V1 with configured Ed25519; or
- manifest V2 with the exact configured algorithm.

The external provider object remains responsible for hardware access and key
custody. No private key enters Core, the machine verifier, package metadata, or
repository files.

For P-256, the Agent normalizes provider output to low-S before filling the
request signature field. The machine verifier rejects any high-S signature
presented from another path.

## Compatibility

- Existing V1 Ed25519 bytes, signer IDs, signatures, capsules, and recovery
  fixtures remain valid.
- Existing ATA slot sizes and request offsets remain unchanged.
- P-256 requires V2 and cannot be interpreted as V1.
- Unknown versions, algorithms, key encodings, and high-S signatures fail
  closed before storage mutation.

## Verification Gates

- Core tests freeze algorithm values, key validation, both signer-ID domains,
  and V1 compatibility.
- Manifest tests freeze exact V1 and V2 bytes and reject cross-version
  encodings.
- Trust tests verify V1 and V2 Ed25519 plus P-256, then reject algorithm
  mismatch, malformed key encodings, invalid curve points, modified manifests,
  and high-S signatures.
- State Signer tests prove policy/provider/manifest agreement.
- Native package tests prove immutable algorithm policy and preserve two
  segments with zero relocations.
- Workspace tests, strict Clippy, Supervisor replay, and both bare-target
  checks pass.

## Exclusions

V18 does not include a TPM transport driver, HSM bus driver, private-key
provisioning ceremony, enabled ATA boot profile, or QEMU execution. Those
layers consume the algorithm-bound contract introduced here.
