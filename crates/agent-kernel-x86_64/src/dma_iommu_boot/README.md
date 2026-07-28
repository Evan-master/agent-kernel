# DMA/IOMMU Proof Profile

This directory owns the dedicated Q35 + Intel VT-d + QEMU EDU boot proof. It
is compiled only with `qemu-dma-iommu-proof`.

## Sequence

```text
discover DMAR + EDU
  -> clear PCI Bus Master
  -> create IOMMU, device, memory, and DMA-domain capabilities
  -> reserve IOVA mapping
  -> program VT-d tables
  -> enable translation
  -> activate mapping
  -> enable PCI Bus Master
  -> verify RAM <-> EDU DMA
  -> begin revocation
  -> remove leaf + invalidate
  -> complete revocation
  -> verify blocked DMA fault + unchanged memory
```

## Modules

| Module | Responsibility |
| --- | --- |
| `authority.rs` | Core resources, capabilities, domain, and mapping lifecycle |
| `memory.rs` | BSP-owned table and DMA page allocation |
| `pci.rs` | exact EDU selection, BAR validation, and command gating |
| `../dma_iommu_boot.rs` | proof orchestration and serial evidence markers |

The proof treats QEMU exit status `33` as success. Any unexpected DMA result,
fault source, IOVA, access direction, or fault reason terminates through a
typed error marker. Failures after activation first attempt a verified PCI Bus
Master shutdown.
