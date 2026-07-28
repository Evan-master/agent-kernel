# Native DMA/IOMMU V27 Implementation Plan

## 1. Core DMA Authority

- [x] Add typed DMA mapping records, lifecycle states, access modes, and requester
  identities.
- [x] Add IOMMU and DMA-domain Resource kinds.
- [x] Store domains, attachments, and mappings in deterministic fixed-capacity
  arrays.
- [x] Add Capability-checked domain creation, device attachment, mapping
  reservation, activation, cancellation, revocation, and release.
- [x] Add Event kinds for every DMA state mutation.
- [x] Expose the lifecycle through the `agent-kernel` syscall-style facade.

## 2. ACPI DMAR

- [x] Add strict byte-oriented DMAR parsing.
- [x] Validate SDT signature, declared length, checksum, reserved fields, structure
  lengths, DRHD register alignment, and bounded device scopes.
- [x] Reuse the existing RSDP and RSDT/XSDT discovery path.
- [x] Select the remapping unit that covers the fixed EDU requester.

## 3. Intel VT-d

- [x] Add legacy root, context, and 39-bit second-stage table encoders.
- [x] Add typed volatile MMIO access for version, capability, root pointer,
  command/status, invalidation, and fault registers.
- [x] Add bounded polling and explicit errors.
- [x] Add leaf install/remove operations and cache invalidation.

## 4. QEMU EDU

- [x] Discover exact PCI identity `1234:11e8` at `00:05.0`.
- [x] Probe BAR0 and map its MMIO window.
- [x] Keep PCI memory decode and bus mastering disabled through setup.
- [x] Add typed EDU DMA registers and bounded completion polling.

## 5. Native Closed Loop

- [x] Add the `qemu-dma-iommu-proof` feature and dedicated boot flow.
- [x] Create semantic Resources and Capabilities before hardware activation.
- [x] Prove a permitted 64-byte round trip.
- [x] Revoke the mapping and prove a blocked transfer through the VT-d fault
  record and unchanged destination bytes.
- [x] Emit stable evidence markers and exit through `isa-debug-exit`.

## 6. Gates

- [x] Run focused tests after each layer.
- [x] Run workspace tests and strict host/bare Clippy.
- [x] Run debug and release V27 QEMU proofs.
- [x] Run debug and release default QEMU regressions.
- [x] Run formatting, Supervisor replay, scripts, and repository audits.
- [x] Update both READMEs and local architecture notes.
