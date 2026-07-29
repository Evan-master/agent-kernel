# Native IPv4/UDP Network Driver Agent V30

## Goal

V30 proves that a Ring-3 Driver Agent can request a capability-authorized UDP
exchange while the kernel retains ownership of Virtio queues, DMA mappings,
MSI-X delivery, and packet validation.

The QEMU proof sends one fixed UDP datagram from `10.0.2.15` to the user-network
gateway at `10.0.2.2`, receives the echoed payload, records both directions in
Core, and then tears down the endpoint and DMA authority.

## Boundaries

| Owner | Responsibilities |
| --- | --- |
| Core | IPv4/UDP value validation, endpoint authority, transfer lifecycle, semantic datagram evidence |
| Facade | `no_std` syscall surface for datagram transmit and receive records |
| Ring-3 Driver Agent | Inspect invocation, acknowledge event, submit a bounded network command, complete invocation |
| x86_64 backend | Ethernet/ARP/IPv4/UDP wire codec, Virtio queues, MSI-X, VT-d, checksum verification |
| QEMU harness | Host UDP echo service and deterministic evidence checks |

Ring-3 receives no MMIO address, DMA frame, page-table root, or interrupt
controller handle. Its command identifies a predeclared operation. The backend
maps that operation to resources already bound through Core capabilities.

## Core Contract

`NetworkIpv4Address` accepts canonical unicast addresses. It rejects unspecified,
limited-broadcast, multicast, and loopback values.

`NetworkUdpPort` accepts ports `1..=65535`.

`NetworkDatagramDescriptor` records:

- source and destination IPv4 addresses;
- source and destination UDP ports;
- payload length;
- SHA-256 payload digest.

A datagram transfer also carries its Ethernet frame descriptor. Core requires
EtherType `0x0800` and the canonical Ethernet + IPv4 + UDP frame size, including
Ethernet minimum-frame padding.

Datagram transmit follows `Prepared -> Completed | Failed`. Receive evidence is
inserted as completed. Every mutation emits one typed event.

## Driver Protocol

The Network Driver Agent executes the established five-call transcript:

1. `DescribeContext`
2. `InspectDriverInvocation`
3. `AcknowledgeDeviceEvent`
4. `SubmitDriverCommand`
5. `CompleteDriverInvocation`

V30 uses two bounded invocations:

| Invocation | Driver opcode | Backend effect |
| --- | --- | --- |
| Neighbor resolution | `0x3001` | Transmit ARP request and verify the gateway reply |
| UDP exchange | `0x3002` | Transmit one IPv4/UDP datagram and verify one echoed reply |

Each invocation gets a fresh Ring-3 address space. The capsule selects the
command from the delivered device-event code and cannot alter endpoint identity,
addresses, ports, or payload bytes.

## Wire Contract

- Ethernet II
- IPv4 version 4, IHL 5, no fragmentation, protocol 17
- IPv4 header checksum required
- UDP checksum required and verified with the IPv4 pseudo-header
- fixed guest address `10.0.2.15`
- fixed gateway address `10.0.2.2`
- fixed source port `40131`
- fixed destination port `40130`
- fixed payload `AGENT-V30-UDP`

Malformed replies fail closed and produce a profile-specific diagnostic marker.

## Verification

Host tests cover:

- Core value and lifecycle contracts;
- capability denial and atomic failure;
- IPv4 header and UDP checksum vectors;
- strict decode rejection for malformed, fragmented, misaddressed, and
  payload-mismatched frames;
- Ring-3 capsule command selection.

QEMU proves:

- ARP resolution through the Driver Agent;
- UDP transmit and echoed receive through the Driver Agent;
- RX and TX MSI-X delivery;
- Core datagram evidence;
- endpoint release and VT-d teardown.
