# Native MSI/MSI-X Proof

Dedicated Q35 boot path for the V28 multi-device interrupt and DMA milestone.
It is compiled only with `qemu-msi-msix-proof`.

## Fixed Topology

| Function | BDF | Requester | Route |
| --- | --- | --- | --- |
| QEMU EDU | `00:05.0` | `0x28` | MSI vector `0xd0` |
| modern virtio-rng | `00:06.0` | `0x30` | MSI-X entry 0, vector `0xd1` |

Both functions attach to VT-d Domain 1. EDU data, split-ring metadata, and
entropy output use three adjacent 4 KiB IOVAs in one bounded 2 MiB window.

## Ownership

| Module | Responsibility |
| --- | --- |
| `../msi_msix_boot.rs` | activation order, proof orchestration, evidence |
| `authority.rs` | Core Device, DMA, Memory, and Interrupt Route lifecycle |
| `pci.rs` | exact identities, BARs, MSI/MSI-X setup, Bus Master gate |
| `memory.rs` | exclusive translation-table and DMA frames |
| `interrupts.rs` | fixed IDT ingress counters for vectors `0xd0` and `0xd1` |

## Proof Order

```text
IDT + Local APIC
  -> PCI discovery and quiescence
  -> Core authority reservation
  -> shared VT-d tables
  -> MSI and MSI-X configuration
  -> EDU MSI completion
  -> virtio-rng MSI-X completion
  -> virtio requester detach
  -> requester-specific VT-d fault
  -> EDU survivor completion
  -> route release and hardware shutdown
```

Every post-activation failure first attempts to clear Bus Master on both
functions. Device causes are acknowledged before Local APIC EOI.
