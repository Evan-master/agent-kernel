# PCI Resource Claims V22 Implementation Plan

**Status:** complete

## 1. Freeze Contracts

- [x] Separate read-only PCI configuration access from mutation authority.
- [x] Define the reversible Type-0 BAR probe.
- [x] Define the fixed BAR catalog and claim mapping.
- [x] Define the atomic Driver Resource Tree transaction.

## 2. Drive With Contracts

- [x] Add failing PCI configuration-write tests.
- [x] Add failing BAR sizing and restoration tests.
- [x] Add failing Resource Tree atomicity tests.
- [x] Add failing PCI claim mapping tests.

## 3. Implement Native PCI Resources

- [x] Add the 32-bit configuration data-write adapter.
- [x] Implement BAR types, sizing, validation, and restoration.
- [x] Build the deterministic fixed-capacity function-resource catalog.

## 4. Implement Kernel Authority

- [x] Add fixed Resource Tree request and outcome values.
- [x] Preflight every authority, range, overlap, capacity, and event condition.
- [x] Commit root and region records with ordered audit events.
- [x] Expose the transaction through the syscall facade.

## 5. Integrate Native Boot

- [x] Retain the BAR catalog and one deterministic function claim in the BSP.
- [x] Reserve one root plus six region slots outside the Agent workflow quota.
- [x] Add boot evidence and QEMU marker assertions.
- [x] Update bilingual status, architecture map, and roadmap.

## 6. Verify And Publish

- [x] Run focused and full workspace tests.
- [x] Run host and freestanding strict Clippy.
- [x] Run supervisor, shell, package, image, and assembly gates.
- [x] Commit V22, push its branch, and verify the remote SHA.

## Gate Record

```text
workspace tests     pass
strict Clippy       host + x86_64-unknown-none
native image audit  8 images + 4 assembly sources
QEMU                debug + release / Events 1..417
terminal marker     SUPERVISOR_HANDOFF_READY
```
