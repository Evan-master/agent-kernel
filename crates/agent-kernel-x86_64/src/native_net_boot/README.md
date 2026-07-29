# Native Network Proof

Dedicated Q35 boot path for the V29 Virtio network, MSI-X, and VT-d milestone.
It is compiled only with `qemu-native-net-proof`.

## Fixed Topology

| Element | Value |
| --- | --- |
| Function | modern virtio-net at `00:05.0` |
| Identity | MAC `52:54:00:12:34:56` |
| RX route | MSI-X entry 0, vector `0xd2` |
| TX route | MSI-X entry 1, vector `0xd3` |
| Gateway | QEMU user network `10.0.2.2` |
| DMA domain | VT-d Domain 1, four 4 KiB IOVA leaves |

## Ownership

| Module | Responsibility |
| --- | --- |
| `../native_net_boot.rs` | Activation order and proof orchestration |
| `authority.rs` | Core Device, DMA, route, endpoint, transfer lifecycle |
| `pci.rs` | Exact function, BARs, MSI-X table, Bus Master gate |
| `pci/discovery.rs` | DMAR requester and PCI region validation |
| `memory.rs` | Exclusive translation-table and queue frames |
| `interrupts.rs` | Fixed IDT ingress counters for both queue vectors |
| `network_proof.rs` | Frame digests and detached-DMA negative probe |
| `proof.rs` | Ordering barriers, fault polling, emergency quiescence |

## Proof Order

```text
IDT + Local APIC
  -> PCI discovery with Bus Master clear
  -> Core authority reservation
  -> VT-d context + four leaves
  -> two MSI-X routes
  -> endpoint activation
  -> ARP transmit + TX completion
  -> ARP reply + RX completion
  -> endpoint and requester release
  -> detached TX notification
  -> exact VT-d fault + IF-enabled zero-interrupt observation
  -> mapping release + IOMMU shutdown
```

The script accepts QEMU status `33` only after
`AGENT_KERNEL_NATIVE_NET_PROOF_OK`. Every post-activation fatal path attempts
to quiesce the PCI function before exiting.
