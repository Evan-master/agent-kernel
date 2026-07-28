# Native MSI/MSI-X and Multi-Device DMA V28 Implementation Plan

## 1. Core Authority

- [x] Add Interrupt Route Resource kind, records, modes, targets, and lifecycle.
- [x] Add Capability-checked route reservation, activation, two-phase
  revocation, and release.
- [x] Add route uniqueness, capacity, event, and retirement constraints.
- [x] Add DMA attachment status plus two-phase requester detachment.
- [x] Expose both lifecycles through the `agent-kernel` facade.

## 2. PCI Message Interrupts

- [x] Add bounded conventional PCI capability traversal.
- [x] Decode MSI, MSI-X, and Virtio vendor capabilities.
- [x] Add typed xAPIC MSI message encoding.
- [x] Add verified MSI configuration and disable paths.
- [x] Add bounded MSI-X table programming, masking, and function control.

## 3. Shared VT-d Domain

- [x] Support multiple requester contexts on one bus and Domain.
- [x] Support multiple 4 KiB leaves inside one 2 MiB IOVA window.
- [x] Remove one requester without disturbing shared mappings.
- [x] Remove one leaf without disturbing other mappings.
- [x] Add table encoding, capacity, isolation, and rebinding tests.

## 4. Modern Virtio Entropy

- [x] Discover exact non-transitional virtio-rng identity and BAR layout.
- [x] Parse Common, Notify, and ISR vendor capabilities.
- [x] Negotiate Version 1 and Access Platform.
- [x] Configure one split virtqueue with IOVA addresses.
- [x] Bind queue completion to MSI-X entry zero.

## 5. Native Closed Loop

- [x] Add two native IDT message-interrupt handlers and Local APIC EOI.
- [x] Add the `qemu-msi-msix-proof` feature and dedicated boot flow.
- [x] Attach EDU and virtio-rng to one Core DMA Domain.
- [x] Prove EDU DMA completion through MSI.
- [x] Prove virtio entropy completion through MSI-X.
- [x] Detach virtio-rng and prove requester-specific VT-d denial.
- [x] Prove EDU remains operational in the shared Domain.
- [x] Emit stable evidence markers and exit through `isa-debug-exit`.

## 6. Gates

- [x] Run focused tests after each layer.
- [x] Run workspace tests and strict host/bare Clippy.
- [x] Run debug and release V28 QEMU proofs.
- [x] Run debug and release V27 QEMU regressions.
- [x] Run debug and release default QEMU regressions.
- [x] Run formatting, Supervisor replay, scripts, and repository audits.
- [x] Update both READMEs and local architecture notes.
