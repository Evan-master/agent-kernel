# Native TPM State Signer V19 Design

Status: implemented

## Objective

V19 gives the native State Signer a kernel-mediated TPM 2.0 signing path.
The durable-state private key remains inside a TPM. Ring 3 receives one
bounded signature result and never receives TPM MMIO authority, a key blob, or
an unrestricted TPM command channel.

## Standards Baseline

- [TCG ACPI Specification 1.5](https://trustedcomputinggroup.org/resource/tcg-acpi-specification/),
  published 2025-12-12
- [TCG PC Client Platform TPM Profile 1.07](https://trustedcomputinggroup.org/resource/pc-client-platform-tpm-profile-ptp-specification/),
  published 2026-03-23
- [TPM 2.0 Library Specification 185](https://trustedcomputinggroup.org/resource/tpm-library-specification/),
  published 2026-03-12

The wire layer supports `TPM2_SignDigest` from version 185 and the established
`TPM2_Sign` command for provisioned TPMs implementing earlier library
revisions. The selected command is immutable provisioning metadata.

## Trust Boundary

```text
State Signer Agent (CPL3)
  -> Agent Call 56: SignDurableArchive
     -> scheduler identity authentication
     -> active durable preparation match
     -> provisioned signer/policy match
     -> kernel-owned TPM command service
        -> TPM2_ReadPublic boot verification
        -> SHA-256(manifest)
        -> TPM2_SignDigest or TPM2_Sign
     -> canonical 64-byte low-S P1363 signature
  -> Agent Call 55: CommitDurableArchiveFromMemory
```

Only the manifest already staged by Agent Call 54 may be signed. Call 56
contains a call-data generation, not a user pointer or arbitrary digest.

## Layer Placement

| Layer | Ownership |
| --- | --- |
| `agent-kernel-hal` | allocation-free TPM command transport trait |
| `agent-kernel-x86_64::tpm2` | ACPI table parsing, CRB state machine, TPM wire codec, provisioned signer |
| x86 bare runtime | MMIO mapping, volatile CRB access, signer service ownership |
| Agent Call ABI | operation 56 decode, authentication, canonical reply |
| `agent-state-signer/native` | ring-3 provider shim that requests Call 56 |
| `docs/` | offline provisioning ceremony and immutable record format |

Core receives no TPM types. TPM command details remain outside the
architecture-independent authorization model.

## ACPI Discovery

The parser accepts a complete, checksum-valid `TPM2` SDT with revision 4
through 6 and at least 52 bytes. It requires:

- zero platform reserved field;
- nonzero CRB control-area physical address;
- Start Method 7;
- no arithmetic overflow while deriving the locality-zero register page.

Start Method 8 is rejected until the kernel has an AML executor for the ACPI
Start method. FIFO, I2C, SMC/HVC, FF-A, AMD mailbox, and vendor methods are
outside this x86 CRB transport.

The ACPI control-area address identifies `TPM_CRB_CTRL_REQ_0`. Locality
registers are derived from the PTP-defined offsets preceding that address.

## CRB Transport

`TpmCommandTransport` executes one caller-bounded command into one
caller-owned response buffer. The CRB implementation:

1. requires active `InterfaceType=1`, `InterfaceVersion=0..=3`, `CapCRB=1`,
   zero reserved bits, and valid buffer descriptors;
2. requests locality 0 and waits for `Granted`;
3. requests `cmdReady` and waits for `tpmIdle == 0`;
4. writes command bytes sequentially from the command-buffer base;
5. writes only the `Start` bit and polls until the TPM clears it;
6. reads and validates the ten-byte TPM response header before the body;
7. writes `goIdle`, waits for idle, and relinquishes locality;
8. clears cancel and relinquishes locality on every recoverable error path.

All waits use explicit poll budgets. Fatal status, timeout, seizure, truncated
response, oversized command/response, changing descriptors, or malformed TPM
headers fail closed. V19 handles one-buffer commands only; neither supported
sign command approaches the CRB chunk boundary.

The RAM-buffer interface (`InterfaceType=2`) remains outside V19. Command and
response descriptors must remain inside the locality-zero data window from
offset `0x80` through the end of the 4 KiB register page.

## Provisioned Signer Record

The immutable boot record contains:

```text
persistent_handle       u32, 0x81000000..0x81ffffff
digest_sign_command     SignDigestV185 | SignV184
policy_generation       nonzero u64
expected_name           34 bytes: TPM_ALG_SHA256 || SHA256(TPMT_PUBLIC)
expected_public_key     33-byte compressed SEC1 P-256 point
```

The signer ID is derived through the existing
`durable_state_signer_id_for_key` function. No duplicate signer ID is accepted
from configuration.

Boot initialization issues `TPM2_ReadPublic` and requires:

- returned Name equals the configured Name;
- returned Name equals `TPM_ALG_SHA256 || SHA256(TPMT_PUBLIC)`;
- object type ECC, curve NIST P-256, and name algorithm SHA-256;
- ECDSA with SHA-256, null symmetric algorithm, and null KDF;
- fixed TPM, fixed parent, sensitive-data-origin, user-auth, and signing bits;
- restricted, decrypt, and X.509-sign bits clear;
- empty authorization policy;
- exact public-point equality with the configured compressed key.

The provisioning ceremony creates an unrestricted signing key with empty
object auth. A later policy-session milestone can bind signing to measured
boot state without putting an authorization secret in the kernel image.

## Offline Provisioning Ceremony

Run the ceremony on the target machine from an isolated administrative
environment. Confirm that the selected persistent handle is vacant. Supply
site-specific owner authorization and TCTI options through protected
`tpm2-tools` mechanisms.

```console
$ umask 077
$ tpm2_createprimary -C o -g sha256 -G ecc -c parent.ctx
$ tpm2_create -C parent.ctx -g sha256 \
    -G ecc256:ecdsa-sha256 \
    -a 'fixedtpm|fixedparent|sensitivedataorigin|userwithauth|sign' \
    -u state-signer.pub -r state-signer.priv
$ tpm2_load -C parent.ctx \
    -u state-signer.pub -r state-signer.priv \
    -c state-signer.ctx
$ tpm2_evictcontrol -C o -c state-signer.ctx \
    -o state-signer.handle 0x81010001
$ tpm2_readpublic -c 0x81010001 \
    -n state-signer.name -o state-signer.pem -f pem
```

Render the public boot fields and derived durable signer ID:

```console
$ scripts/inspect-tpm-state-signer.rb \
    --handle 0x81010001 \
    --command sign-digest-v185 \
    --policy-generation 1 \
    --name state-signer.name \
    --public-key state-signer.pem
```

Select `sign-digest-v185` when the TPM advertises
`TPM2_CC_SignDigest`; select `sign-v184` for a provisioned TPM that exposes
`TPM2_Sign`. Bind the same public key, signer ID, root, and policy generation
into `NativeAtaDurableConfig`.

The inspector accepts public material only. It rejects private PEM input,
non-P256 keys, malformed Names, invalid handles, and zero generations. Boot
still performs `TPM2_ReadPublic` and validates the complete public template.

## TPM Wire Contract

All TPM integers use big-endian encoding. Commands and responses use
fixed-capacity stack buffers.

`TPM2_ReadPublic` has no authorization session. Signing uses one
`TPM_RS_PW` authorization with empty nonce and empty HMAC. The digest is
SHA-256 over the exact 285-byte durable manifest. The null validation ticket
uses `TPM_ST_HASHCHECK`, `TPM_RH_NULL`, and an empty digest.

The response parser accepts only ECDSA/SHA-256 with P-256-sized `r` and `s`.
It rejects zero or out-of-range scalars, trailing parameter bytes, malformed
authorization responses, and non-success TPM return codes. Valid high-S
results are normalized before conversion to the kernel's 64-byte IEEE P1363
format.

## Agent Call 56

Request payload:

```text
rax magic
rbx ABI version
rcx 56
rdx flags = 0
rsi/rdi/r8/r9 authenticated Agent context
r10 call-data generation
r11/r12/r13/r14/r15/rbp = 0
```

The handler requires:

- authenticated caller identity;
- a live Call 54 preparation owned by the same Agent, Task, and image;
- exact call-data generation;
- a State Signer entry and image;
- signer algorithm P-256/SHA-256;
- exact signer ID and policy generation match;
- an unsigned canonical request still present in the private call-data page.

The kernel signs its retained manifest, rechecks the page snapshot, attaches
the signature, and replaces the full fixed-width request. A successful reply
returns generation, signer policy generation, and algorithm identity. TPM
status details remain privileged.

Call 56 records its operation in the immutable Agent Call transcript. It emits
no Core Event because it changes only the stopped caller's transient private
page; Call 55 owns the durable mutation and Event archive checkpoint.

## Failure Semantics

- No automatic fallback to software private keys.
- No fallback from one provisioned persistent handle to another.
- No arbitrary TPM command Agent Call.
- No signing after cancellation, task fault, identity change, or stale
  generation.
- No commit if the signed page differs from the prepared manifest.
- A boot-time key-binding mismatch stops boot.
- A runtime transport, wire, or signature-verification failure permanently
  disables the signer instance for that boot.

## Validation

- ACPI parser fixtures for revisions, checksums, methods, and overflow
- scripted CRB register model covering success, timeout, seizure, fatal state,
  cleanup, and changing descriptors
- byte-exact ReadPublic and both signing-command codec tests
- key-template, Name, point, signature, and authorization-response rejection
- Agent Call 56 decode/authentication/transcript tests
- ring-3 provider object audit and package test
- fake TPM closed loop through prepare, hardware-sign call, ATA commit, power
  loss, and cold recovery
- workspace tests, strict Clippy, supervisor run, package test, and both
  freestanding builds

## Implementation Record

```text
HAL transport       allocation-free TpmCommandTransport
ACPI                root discovery + strict TPM2 parser
CRB                 locality / ready / command / cleanup / poison
wire                 ReadPublic + SignDigest v185 + Sign v184
boot binding         supervisor-only uncached MMIO + ReadPublic verification
Agent ABI            Call 56 + authenticated retained-manifest service
ring 3               built-in Package v3 TPM provider
recovery             fake TPM -> ATA A/B commit -> power loss -> cold boot
```
