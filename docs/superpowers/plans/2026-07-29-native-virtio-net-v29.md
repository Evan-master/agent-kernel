# Native Virtio Network V29 Implementation Plan

## Core authority

- [x] Add validated MAC, endpoint configuration, frame descriptor, endpoint
      lifecycle, and transfer lifecycle values.
- [x] Add fixed-capacity endpoint and transfer stores to `KernelCore`.
- [x] Add Capability-checked endpoint reserve/activate/revoke methods.
- [x] Add two-phase transmit and audited receive methods.
- [x] Add Event tags, errors, retirement guards, facade methods, and tests.

## Virtio network driver

- [x] Add exact modern Virtio network PCI capability selection.
- [x] Add bounded device-configuration MMIO for the MAC address.
- [x] Add one-descriptor RX and TX split queues.
- [x] Add two-queue feature negotiation and MSI-X assignment.
- [x] Add an ordered device owner with reset-safe shutdown.
- [x] Add focused transport, queue, PCI, and MMIO contract tests.

## Native proof

- [x] Add the V29 bare-metal feature and mutually exclusive boot branch.
- [x] Reserve Network, Device, DMA, Memory, and Interrupt Route authority.
- [x] Discover and quiesce one exact Q35 Virtio network function.
- [x] Program two MSI-X entries and four VT-d leaves.
- [x] Transmit an ARP request and validate the gateway reply.
- [x] Revoke the endpoint and prove requester-level DMA denial.
- [x] Add Debug and Release QEMU build/run scripts with exact markers.

## Delivery

- [x] Update local architecture READMEs and bilingual root README.
- [x] Run formatting, full workspace tests, strict Clippy, image audit, and
      supervisor replay.
- [x] Run default, V27, V28, and V29 bare-metal checks.
- [x] Run V29 Debug and Release QEMU proofs.
- [x] Commit and push `feature/native-virtio-net-v29`.
