# Native TPM State Signer V19 Implementation Plan

Status: implemented

## 1. Freeze Contracts

- Add the HAL TPM command transport trait and contract tests.
- Add typed provisioning values and stable error enums.
- Add pure ACPI `TPM2` table parser tests.

## 2. Implement CRB

- Add register-I/O abstraction and descriptor validation.
- Implement bounded locality, ready, start, response, idle, and cleanup states.
- Add the volatile x86 MMIO adapter.
- Add a boot-owned mapping helper for the locality-zero CRB window.

## 3. Implement TPM Wire Codec

- Add fixed-capacity big-endian encoder and decoder.
- Encode and parse `TPM2_ReadPublic`.
- Encode `TPM2_SignDigest` and `TPM2_Sign`.
- Parse ECDSA P-256 responses and normalize low-S.

## 4. Bind Provisioned Hardware

- Validate persistent handle, expected Name, policy generation, and public key.
- Verify the full public template during initialization.
- Expose a disabled/provisioned boot profile with no secret material.
- Document the offline key-provisioning ceremony.

## 5. Add Native Agent Path

- Add Agent Call operation 56 and register envelope tests.
- Extend authentication, transcript, reply, and private call-data handling.
- Add the kernel-owned hardware signer service to the native executor.
- Add the ring-3 TPM provider shim and package audit coverage.

## 6. Prove Recovery

- Run a fake TPM signer through durable preflight and ATA transaction.
- Simulate power loss and cold recovery using the same provisioned public key.
- Add failure cases for stale generation, wrong caller, wrong key, and TPM
  transport failure.

## 7. Publish

- Update English and Chinese README status matrices and roadmap.
- Run formatting, workspace tests, strict Clippy, supervisor, package, and
  freestanding build gates.
- Inspect the diff, commit V19, push the branch, and verify the remote SHA.

## Completion Evidence

```text
contracts      HAL / ACPI / wire / Agent Call 56
transport      CRB locality 0 + volatile MMIO + poison semantics
signer         ReadPublic template binding + P-256 low-S verification
ring 3         built-in TPM provider in signed Package v3
recovery       TPM signature -> ATA commit -> power loss -> cold recovery
tooling        public provisioning inspector + package audit
```
