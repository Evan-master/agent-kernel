# TPM Measured Policy V20 Design

**Status:** implemented and verified in the scripted TPM boundary

## Objective

V20 binds durable-state signatures to the measured boot state held by a TPM
2.0 SHA-256 PCR bank. The signing key carries an immutable `authPolicy`.
Every signature request must reproduce that policy inside a fresh TPM policy
session.

The policy has two assertions:

1. `TPM2_PolicyPCR` checks one configured PCR composite digest.
2. `TPM2_PolicyCommandCode` limits the session to the configured signing
   command.

The TPM private key remains non-exportable. Agents retain the existing Call 56
surface and receive no TPM session handle or raw command authority.

## Standards Basis

- [TPM 2.0 Library Part 1, version 185](https://trustedcomputinggroup.org/resource/tpm-library-specification/)
- [TPM 2.0 Library Part 2, version 185](https://trustedcomputinggroup.org/resource/tpm-library-specification/)
- [TPM 2.0 Library Part 3, version 185](https://trustedcomputinggroup.org/resource/tpm-library-specification/)

The implementation uses:

- `TPM2_StartAuthSession`
- `TPM2_PolicyPCR`
- `TPM2_PolicyCommandCode`
- `TPM2_SignDigest` or `TPM2_Sign`
- `TPM2_FlushContext`

## Trust Boundary

```text
signed boot policy
  -> expected TPM Name
  -> expected P-256 public point
  -> selected PCR bitmap + expected composite digest
  -> computed object authPolicy

Agent Call 56
  -> retained manifest validation
  -> SHA-256 manifest digest
  -> kernel-owned TPM policy session
  -> TPM signature
  -> kernel signature verification
```

Ring 0 owns policy construction, session handles, CRB transport, and cleanup.
Ring 3 can request the existing retained-manifest operation only.

## Policy Configuration

`Sha256PcrPolicy` contains:

| Field | Encoding |
| --- | --- |
| PCR selection | three-byte bitmap for PCR 0 through PCR 23 |
| expected PCR digest | SHA-256 of selected PCR values in TPM bank order |

An empty PCR selection is invalid. V20 supports exactly one SHA-256 bank.
Multiple banks and dynamic policy authorization remain future work.

The signer supports two authorization modes:

| Mode | Public template | Runtime authorization |
| --- | --- | --- |
| empty password | empty `authPolicy`, `userWithAuth` set | `TPM_RS_PW` |
| measured PCR policy | computed 32-byte `authPolicy`, `userWithAuth` clear, `adminWithPolicy` set | policy session |

The existing constructor selects empty-password mode. A dedicated constructor
selects measured-policy mode.

## Policy Digest

All integers use big-endian TPM wire encoding. The PCR selection is marshalled
as one `TPML_PCR_SELECTION` entry:

```text
count          UINT32  1
hash           UINT16  TPM_ALG_SHA256
sizeofSelect   UINT8   3
pcrSelect      BYTE[3]
```

The object authorization digest is:

```text
d0 = 32 zero bytes
d1 = SHA256(d0 || TPM_CC_PolicyPCR || marshalled_selection || pcr_digest)
d2 = SHA256(d1 || TPM_CC_PolicyCommandCode || signing_command_code)
```

`d2` must equal the public area's `authPolicy`. The command code is
`TPM_CC_SignDigest` for version-185 mode and `TPM_CC_Sign` for version-184
mode.

## Session Protocol

Each measured-policy signature uses a fresh session:

```text
StartAuthSession(policy, SHA-256, symmetric NULL)
  -> policy session handle
PolicyPCR(handle, expected PCR digest, selection)
PolicyCommandCode(handle, configured signing command)
Sign(handle authorization, continueSession = 1)
FlushContext(handle)
```

`tpmKey` and `bind` are `TPM_RH_NULL`. The session is unsalted and unbound.
The policy contains no `PolicyAuthValue`, so command and response HMAC fields
are empty as permitted by the TPM architecture specification. Parameter
encryption is unused.

The initial caller nonce is 16 bytes, derived from the manifest digest. It
provides the required wire size and per-request variation. No security claim
depends on caller nonce secrecy because the session key is empty.

The signing authorization sets `continueSession`. This keeps cleanup
deterministic across successful and failed signing responses. Ring 0 issues
`FlushContext` after every signing attempt.

## Parsing and Bounds

All command layouts have fixed upper bounds:

| Command | Bytes |
| --- | ---: |
| `StartAuthSession` | 43 |
| `PolicyPCR` | 58 |
| `PolicyCommandCode` | 18 |
| signing | 71 |
| `FlushContext` | 14 |

The response parser requires:

- exact response length;
- expected command tag;
- zero response code;
- a policy-session handle;
- a 16-byte initial TPM nonce;
- empty policy authorization HMAC;
- matching session attributes;
- no trailing bytes.

The empty-password signature response follows the TPM rule that
`continueSession` is set in the password authorization acknowledgment.

## Failure Semantics

Any runtime TPM transport, wire, policy, cleanup, or signature-verification
failure disables the signer for the rest of the boot.

Once a session handle exists, all failure paths attempt `FlushContext`.
Cleanup failure is fail-closed and also disables the signer. A failed
`StartAuthSession` has no known session handle to flush.

PCR mismatch, PCR update-counter change, wrong command code, and wrong object
policy surface as TPM response codes and enter the same disabled state.

## Verification

V20 requires:

- frozen policy-digest vectors for both signing command codes;
- byte-exact command tests;
- strict response parser tests;
- public-template mode and policy mismatch tests;
- successful measured-policy signer sequence;
- PCR rejection and cleanup tests;
- cleanup-failure and transport-length tests;
- durable archive, ATA commit, power-loss, and cold-recovery closed loop;
- workspace formatting, tests, clippy, supervisor, package audit, and bare
  target checks.

Physical TPM execution remains a hardware validation step after the scripted
state-machine proof.
