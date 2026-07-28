# Modern Virtio RNG

Native Virtio 1.x entropy-device support for the x86_64 architecture layer.

## Ownership

| Module | Responsibility |
| --- | --- |
| `pci.rs` | Exact PCI identity, vendor-capability selection, BAR resolution |
| `mmio.rs` | Validated volatile Common, Notify, and ISR access |
| `transport.rs` | Feature negotiation, queue setup, MSI-X vector binding |
| `queue.rs` | DMA-visible split-ring encoding and completion validation |
| `device.rs` | Ordered request, interrupt, completion, and shutdown lifecycle |

PCI configuration authority stays in `pci/`. DMA translation authority stays in
`iommu/`. This directory consumes both contracts without owning either one.

## Runtime Contract

```text
PCI discovery
  -> capability validation
  -> BAR mapping
  -> device reset
  -> VERSION_1 + ACCESS_PLATFORM negotiation
  -> queue 0 + MSI-X vector configuration
  -> descriptor publication
  -> Notify write
  -> ISR acknowledgement
  -> used-ring validation
  -> entropy exposure
```

The queue has one descriptor and one in-flight request. Metadata and entropy
buffers occupy separate 4 KiB pages with explicit IOVA addresses. Device-written
lengths, descriptor identifiers, queue indices, and interrupt causes are checked
before bytes become visible to the caller.

Request preparation and Notify are separate state transitions. Authority owners
can publish guard data and open Bus Master only after the complete descriptor
chain is visible, then issue exactly one notification.

## Failure Rules

- Missing, duplicate, misaligned, or out-of-range PCI regions fail before MMIO.
- Required Virtio features must survive the `FEATURES_OK` handshake.
- Queue notification offsets must fit inside the validated Notify region.
- Spurious interrupts and malformed used-ring entries produce typed errors.
- Shutdown resets device status before queue ownership can be reused.

The QEMU proof profile supplies the surrounding PCI, VT-d, MSI-X, IDT, and
interrupt-routing lifecycle.
