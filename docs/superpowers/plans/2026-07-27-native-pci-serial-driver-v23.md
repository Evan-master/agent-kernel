# Native PCI Serial Driver V23 Implementation Plan

**Status:** complete

## 1. Freeze Contracts

- [x] Pin the QEMU PCI serial identity and BDF.
- [x] Define exact target selection and retained-claim checks.
- [x] Define Driver Capability and Binding authority.
- [x] Define bounded 16550 transmit and Event contracts.

## 2. Drive With Tests

- [x] Add failing exact-selector catalog tests.
- [x] Add failing PCI serial endpoint tests.
- [x] Add failing request validation and timeout tests.

## 3. Implement Native Driver

- [x] Add `PciFunctionSelector`.
- [x] Select and claim only the pinned serial Function.
- [x] Add the bounded `PciSerialBackend`.
- [x] Re-admit the completed Worker identity with a BAR-scoped Driver image.
- [x] Delegate the BAR Region Capability to the Driver Agent.
- [x] Execute and record one physical transmit command.

## 4. Integrate QEMU Proof

- [x] Add the pinned `pci-serial` device and file-backed chardev.
- [x] Assert the exact `0x50` physical output byte.
- [x] Extend capacities and exact Event history through 434.
- [x] Add V23 boot markers.

## 5. Verify And Publish

- [x] Run focused and full workspace tests.
- [x] Run host and freestanding strict Clippy.
- [x] Run supervisor, shell, package, image, debug QEMU, and release QEMU gates.
- [x] Update bilingual status and architecture documentation.
- [x] Commit V23, push its branch, and verify the remote SHA.
