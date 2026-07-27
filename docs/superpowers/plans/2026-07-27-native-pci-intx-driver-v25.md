# Native PCI INTx Driver V25 Implementation Plan

**Status:** complete

## 1. Freeze Contracts

- [x] Freeze the QEMU PCI serial INTx profile.
- [x] Define the one-shot IRQ capture and masking lifecycle.
- [x] Define the two-Invocation Driver Capsule behavior.
- [x] Define one-generation Driver fault recovery.
- [x] Preserve Agent Call operations `1..60`.

## 2. Drive With Tests

- [x] Add failing Driver fault and recovery tests.
- [x] Add failing PCI interrupt metadata and route tests.
- [x] Add failing `Configure` Agent Call and backend tests.
- [x] Run the focused tests and confirm intended failures.

## 3. Implement Core Recovery

- [x] Add `Faulted` Driver Invocation state and restart metadata.
- [x] Add fail-before-write fault and owner-authorized recovery transitions.
- [x] Add stable archive tags and event formatters.
- [x] Preserve existing scheduling and command invariants.

## 4. Implement PCI INTx

- [x] Read PCI interrupt line and pin metadata.
- [x] Encode and install the IRQ 11 active-low, level-triggered route.
- [x] Add fixed-capacity PCI serial interrupt capture.
- [x] Add semantic `Configure` backend dispatch.

## 5. Execute And Recover In Ring 3

- [x] Update the auditable Driver Capsule for event-kind dispatch.
- [x] Trap generation zero before any irreversible side effect.
- [x] Clear retained private pages and restart generation one.
- [x] Execute the state-change and real interrupt Invocations.

## 6. Integrate Evidence

- [x] Prove the exact fault, recovery, IRQ, Agent Call, and reclamation evidence.
- [x] Freeze the observed Event suffix and QEMU markers.
- [x] Update image audit counts and release-ELF uniqueness checks.
- [x] Update the bilingual README and V25 status documents.

## 7. Verify And Publish

- [x] Run focused and full workspace tests.
- [x] Run host and freestanding strict Clippy.
- [x] Run supervisor, shell, package, and image audits.
- [x] Run QEMU debug and release with exact Event, IRQ, and `0x50` proofs.
- [x] Commit V25, push its branch, and verify the remote SHA.
