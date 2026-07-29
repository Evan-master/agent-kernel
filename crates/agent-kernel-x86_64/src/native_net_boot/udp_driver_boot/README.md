# Native UDP Driver Proof

V30 composes a Ring-3 Network Driver Agent with the V29 Virtio, MSI-X, and
VT-d owners.

## Ownership

| Module | Responsibility |
| --- | --- |
| `admission.rs` | Driver identity, delegated capability, image, endpoint, binding |
| `backend.rs` | Semantic command dispatch and bounded Virtio exchange |
| `platform.rs` | DMA frames, VT-d tables, MMIO binding, queue initialization |
| `session.rs` | ARP and UDP Core evidence, terminal teardown |
| `session/invocation.rs` | Fresh address space and exact five-call transcript |

## Ring-3 Contract

```text
StateChanged(0x3001) -> Configure / resolve neighbor
StateChanged(0x3002) -> Write / exchange fixed UDP payload
```

The Agent receives resource and event identities through Agent Call replies.
MMIO bases, queue addresses, DMA IOVAs, packet buffers, and APIC state remain
kernel-owned.

## Terminal Invariants

- two completed Driver Invocations;
- one physical quantum expiry per fresh address space;
- exact five-call transcript and generated return offsets;
- two completed commands with matching Event causes;
- completed Core datagram transmit and receive evidence;
- no VT-d fault while attached;
- endpoint released, requester detached, mappings released, VT-d disabled.
