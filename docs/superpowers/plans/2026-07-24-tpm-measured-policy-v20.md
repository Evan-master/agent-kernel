# TPM Measured Policy V20 Implementation Plan

## 1. Freeze the Contract

- Record the TPM 2.0 policy digest formulas and command layouts.
- Define one SHA-256 PCR bank and a three-byte PCR bitmap.
- Preserve the V19 empty-password configuration path.

## 2. Add Policy Primitives

- Add `Sha256PcrPolicy` with nonempty-selection validation.
- Compute `PolicyPCR` plus `PolicyCommandCode` authorization digests.
- Freeze vectors for `SignDigest` and `Sign`.

## 3. Extend the Wire Layer

- Encode `StartAuthSession`, `PolicyPCR`, `PolicyCommandCode`, and
  `FlushContext`.
- Encode signing authorization with a policy-session handle.
- Parse policy-session creation, empty success responses, and policy
  signature acknowledgments.
- Correct the password-session acknowledgment contract.

## 4. Enforce Key Policy

- Add measured-policy signer configuration.
- Require the computed `authPolicy` in `TPMT_PUBLIC`.
- Require policy-only object attributes and reject password bypass.

## 5. Own Session Lifecycle

- Run a fresh policy session for every signature.
- Flush the session after every signing attempt.
- Disable the signer on all runtime and cleanup failures.

## 6. Prove the Closed Loop

- Extend scripted TPM fixtures with policy-aware public areas and responses.
- Test successful signing, PCR rejection, and cleanup.
- Run the policy signer through ATA commit, power loss, and cold recovery.

## 7. Ship

- Update English and Chinese README status panels and roadmap.
- Run all workspace and bare-metal quality gates.
- Inspect the diff, commit V20, push the branch, and verify the remote SHA.
