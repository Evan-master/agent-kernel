# Native PCI Inventory V21 Implementation Plan

**Status:** implemented

## 1. Freeze Contracts

- Define segment-zero BDF and register bounds.
- Freeze Configuration Mechanism 1 selector encoding.
- Define immutable function and class records.

## 2. Build Configuration Access

- Add the fixed PCI I/O trait.
- Implement address-latch probing and restoration.
- Add the native 32-bit `0x0cf8` and `0x0cfc` adapter.

## 3. Build Discovery

- Scan every bus and device in deterministic order.
- Respect the multifunction bit before probing functions 1 through 7.
- Decode the common PCI header into a fixed-capacity inventory.
- Fail closed on empty or overflowing inventories.

## 4. Integrate Boot Ownership

- Retain the inventory in `SmpBootstrap`.
- Discover it before AP startup and Agent execution.
- Add stable serial evidence for configuration access and inventory readiness.

## 5. Verify and Publish

- Run focused PCI tests and full workspace gates.
- Run package, image, assembly, and freestanding audits.
- Update the bilingual status and roadmap.
- Commit V21, push its branch, and verify the remote SHA.
