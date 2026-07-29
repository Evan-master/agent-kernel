# Native Virtio Network V29 Design

## Status

Implemented and verified on `feature/native-virtio-net-v29`.

## Objective

V29 establishes the first capability-governed external network path in Agent
Kernel. One native modern Virtio network device transmits an Ethernet frame,
receives the corresponding reply, delivers both queue completions through
MSI-X, and performs all DMA through an Intel VT-d domain.

The milestone ends at a strict Layer-2 boundary. IP configuration, transport
protocols, name resolution, and application communication APIs remain separate
future layers.

## Core Authority

`agent-kernel-core` owns two fixed-capacity record families:

- `NetworkEndpointRecord` binds one `ResourceKind::Network` child Resource to
  one Device Resource, one unicast MAC address, one MTU, and a lifecycle.
- `NetworkTransferRecord` binds one typed transfer identity to an endpoint,
  direction, bounded Ethernet descriptor, and terminal status.

Endpoint lifecycle:

```text
Reserved -> Active -> Revoking -> Released
```

Transmit lifecycle:

```text
Prepared -> Completed
         \-> Failed
```

Receive records enter directly as `Completed` after the architecture owner has
validated a device-written frame. Data becomes available to an Agent only
after the `Observe` authorization and Event append succeed.

Authority rules:

- Creating an endpoint requires `Act` on the Device Resource.
- Activating and preparing a transmit require `Act` on the Network Resource.
- Recording a receive requires `Observe` on the Network Resource.
- Revocation requires `Rollback` on the Network Resource.
- A prepared transmit blocks endpoint revocation.
- A live endpoint blocks retirement of both the endpoint and its Device.
- Every lifecycle and transfer transition appends one stable Event kind.

Frame evidence is fixed width:

```text
length       Ethernet frame bytes without FCS
ether_type   Ethernet II protocol discriminator
digest       SHA-256 supplied by the trusted architecture owner
```

V29 supports untagged Ethernet frames from 14 bytes through `MTU + 14`, with a
maximum MTU of 1500 bytes.

## Modern Virtio Network Contract

The architecture module selects exactly:

```text
PCI Vendor         0x1af4
PCI Device         0x1041
Virtio Device ID   1
Queue 0            receiveq1
Queue 1            transmitq1
```

Required negotiated features:

```text
VIRTIO_NET_F_MAC
VIRTIO_NET_F_MRG_RXBUF
VIRTIO_F_VERSION_1
VIRTIO_F_ACCESS_PLATFORM
```

V29 excludes checksum offload, segmentation, packed queues, indirect
descriptors, RSS, multiqueue, and a control virtqueue.

Each split queue owns:

- one 16-byte descriptor,
- one available-ring slot,
- one used-ring slot,
- one disjoint 4 KiB packet buffer.

The modern 12-byte `virtio_net_hdr` precedes every packet. TX publishes a fully
checksummed Ethernet frame with no offload flags. RX negotiates mergeable
buffers for the 12-byte QEMU header while exposing one descriptor only. It
requires `num_buffers == 1` and validates the remaining header fields, used
length, and descriptor identity before exposing bytes.

## PCI, Interrupt, And DMA Profile

QEMU Q35 topology:

```text
00:05.0  virtio-net-pci-non-transitional
```

Message routes:

```text
MSI-X table entry 0 -> vector 0xd2 -> receiveq1
MSI-X table entry 1 -> vector 0xd3 -> transmitq1
```

DMA layout:

```text
0x0200_0000  RX split-ring metadata
0x0200_1000  RX packet buffer
0x0200_2000  TX split-ring metadata
0x0200_3000  TX packet buffer
```

VT-d access:

```text
RX metadata   ReadWrite
RX packet     Write
TX metadata   ReadWrite
TX packet     Read
```

Bus Master remains clear until:

1. Core authority is reserved.
2. VT-d context and leaves are present.
3. DMA mappings and interrupt routes are active.
4. BAR regions and MSI-X entries are validated.
5. Both virtqueues are initialized.

## Closed-Loop Proof

The guest MAC is fixed to `52:54:00:12:34:56`. QEMU user networking provides
the local gateway at `10.0.2.2`.

The proof:

1. Posts one RX descriptor.
2. Prepares one Core network transmit.
3. Sends an Ethernet ARP request for `10.0.2.2`.
4. Completes TX through MSI-X entry 1.
5. Receives and validates the ARP reply through MSI-X entry 0.
6. Completes the Core transmit and records the receive.
7. Revokes the endpoint, resets the device, disables MSI-X, and detaches the
   requester.
8. Re-enables only the hardware needed for a denial probe.
9. Notifies a TX descriptor and requires a VT-d translation fault with an
   exact requester ID, an address inside the four-page IOVA window, and no
   completion interrupt throughout an IF-enabled observation window.

Required terminal marker:

```text
AGENT_KERNEL_NATIVE_NET_PROOF_OK
```

## Failure Policy

- All parsing, queue geometry, and feature failures occur before Bus Master.
- Once Bus Master is enabled, every fatal path first quiesces the PCI function.
- Device reset precedes Network Resource release and requester detach.
- MSI-X is disabled before its routes enter `Revoking`.
- IOTLB invalidation separates table mutation from semantic completion.
- Safe MMIO APIs retain bounds checks in optimized builds.

## Compatibility

- The default boot profile is unchanged.
- V27 DMA/IOMMU and V28 MSI/MSI-X profiles remain independently buildable.
- The V29 feature is mutually exclusive with earlier QEMU proof features.
- Existing Event numeric tags and Agent Call values remain stable.

## Verification

| Gate | Result |
| --- | --- |
| Workspace tests | Core, Facade, machine, Supervisor, and doc tests pass |
| Strict Clippy | Host workspace plus four bare-metal profiles |
| Native image audit | 9 images, 2 Package v3 images, 5 assembly sources |
| V29 QEMU Debug | ARP round-trip, MSI-X, VT-d denial, status `33` |
| V29 QEMU Release | ARP round-trip, MSI-X, VT-d denial, status `33` |
| Terminal marker | `AGENT_KERNEL_NATIVE_NET_PROOF_OK` |

## Deferred Work

- IPv4/IPv6 packet construction and validation
- DHCP and static network identity policy
- TCP, UDP, and reliable message channels
- Multiple outstanding descriptors and queue recycling
- Multiqueue and RSS
- A Ring-3 network Driver Agent
- Endpoint delegation to untrusted Agent workloads
- Real NIC drivers and physical hardware validation
