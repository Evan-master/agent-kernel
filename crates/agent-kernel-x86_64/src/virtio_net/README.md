# Modern Virtio Network

Allocation-free Virtio 1.x Ethernet support for the x86_64 architecture layer.

## Ownership

| Module | Responsibility |
| --- | --- |
| `pci.rs` | Exact PCI identity, capability selection, BAR resolution |
| `device_config.rs` | Bounded volatile MAC configuration reads |
| `transport.rs` | Feature negotiation, lifecycle, Notify, ISR handling |
| `transport_queue.rs` | Queue registers, MSI-X vectors, DMA addresses |
| `transport_types.rs` | Protocol constants, observations, typed failures |
| `queue_layout.rs` | Shared one-entry split-ring geometry |
| `rx_queue.rs` | Device-writable buffer publication and validation |
| `tx_queue.rs` | Header encoding and write-free completion validation |
| `device.rs` | Ordered ownership of transport and both queues |
| `arp.rs` | Fixed Ethernet/ARP proof frame contract |

Core owns endpoint and transfer authority. PCI, VT-d, and MSI-X owners supply
the surrounding hardware lifecycle.

## Runtime Contract

```text
exact PCI function
  -> Common + Notify + ISR + Device regions
  -> reset
  -> MAC + MRG_RXBUF + VERSION_1 + ACCESS_PLATFORM
  -> RX queue 0 / MSI-X entry 0
  -> TX queue 1 / MSI-X entry 1
  -> descriptor publication
  -> Notify
  -> MSI-X ingress
  -> used-ring validation
  -> frame evidence
```

Each queue owns one metadata page, one packet page, and one descriptor. The RX
path negotiates mergeable buffers for the modern 12-byte header, then requires
`num_buffers == 1`; no second descriptor can be consumed. Device binding
requires all four RX/TX IOVA pages to be pairwise disjoint.

## Failure Rules

- Missing or duplicate PCI regions fail before MMIO.
- Required features must survive `FEATURES_OK`.
- Queue size, vector, Notify offset, and used identity are read back.
- RX validates header flags, buffer count, frame length, and EtherType.
- RX completion tokens expire before the buffer is published again.
- TX rejects any device write into the packet descriptor.
- Shutdown resets device status before queue reuse.
