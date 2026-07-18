# X86 Native Memory Fault Reclaim V1 Design

## Status

Implemented and validated on 2026-07-18.

## Purpose

Native Memory Concurrency V1 requires every Agent to release its compatibility
page and all runtime regions before completion or fault containment. A ring-3
exception with one live mapping therefore stops the boot proof before the
semantic fault can be recorded.

Fault Reclaim V1 gives the exception path bounded cleanup ownership. Before
committing `TaskFaulted`, the kernel retires every live Memory Resource through
the exact Capability that authorized its allocation, removes the Agent leaves,
zeros and returns the physical frames, and clears the private allocation
ledgers. The captured fault CPU remains restartable after cleanup.

## Authority Binding

Runtime page and region bindings gain the authorizing `CapabilityId`. The
Capability becomes part of reservation, mapping, release, and stale-token
identity. Physical frame ownership remains bound to Agent, Resource,
MemoryCell, generation, and transaction.

The core and facade expose read-only retirement readiness and event-capacity
checks. Fault cleanup uses those public checks before mutation. Resource
retirement still executes only through `sys_retire_resource`.

## Cleanup Transaction

One Faulted CPU can own one compatibility page and up to four region records.
The cleanup adapter builds a fixed five-entry plan in this order:

1. compatibility page, when present;
2. region records in deterministic ledger order.

Preparation validates every entry before any mutation:

- Agent, Resource, Capability, MemoryCell, and generation identity;
- active `Rollback` authority, including the complete parent chain;
- active Memory Resource and exact MemoryCell descriptor;
- private virtual leaf and physical frame agreement;
- live global pool release token;
- enough event slots for every retirement plus `TaskFaulted`;
- first and last physical proof values.

Commit retires all planned Resources through the facade, then deactivates each
private mapping, clears and releases its pool frames, and consumes both virtual
and physical tokens. Single-core execution keeps the prepared state stable
between these phases. Any mismatch follows the existing fail-closed path.

## Reclamation Evidence

The Faulted CPU receives a fixed-capacity reclamation log only after the whole
transaction succeeds. Each entry records:

- page or region kind;
- Resource, Capability, and MemoryCell;
- page count and allocation generation;
- first and last proof values.

Terminal evidence also requires no live runtime leaves, no pool ownership for
the Agent, and zero bytes in every returned frame.

## Fault Worker Proof

Boot setup delegates a dedicated Memory Resource to Fault Worker Agent 6:

| Object | Expected ID |
| --- | ---: |
| Memory Resource | 2 |
| Bootstrap-owned root Capability | 7 |
| Fault Worker Memory Capability | 8 |
| Fault Worker task Capability | 9 |
| MemoryCell | 1 |

The delegated Capability contains `Observe`, `Act`, and `Rollback`. On restart
generation 0, the immutable Worker Capsule performs:

```text
DescribeContext
    -> AllocateMemoryRegion(capability 8, resource 2, two pages)
    -> require base 0x0000400000009000, 0x2000 bytes, cell 1, generation 1
    -> write first proof 0x4641554c544d3031
    -> write last proof  0x4641554c544d3032
    -> execute ud2 with the region still live
```

The first contained fault must carry a two-call transcript and one reclamation
entry. The existing restart generations 1 through 3 continue to prove `#GP`,
write-protection `#PF`, demand-page repair, Fault Handler routing, and final
completion.

## Capsule Artifacts

The immutable Capsule arrays are generated from adjacent auditable assembly and
bound to exact release-ELF bytes:

| Capsule | Total bytes | Code bytes | SHA-256 |
| --- | ---: | ---: | --- |
| Fault Worker | 325 | 293 | `a74bdafa93cb878d578b2dd75ff9b6000d0f6e96ab39d01d658496821aedc4de` |
| Resource Manager | 2688 | 2656 | `2d41543aeb6d09b5f50b65a333f05e35340311b2e74bebf18f9d03024de3477d` |

Release extraction, byte comparison against compiled `.S` sections, and x86_64
disassembly form part of milestone validation.

## Deterministic Capacity Changes

The extra delegated Memory Resource shifts later Manager-created handles:

| Manager object | Resource | Capability | MemoryCell |
| --- | ---: | ---: | ---: |
| Service | 3 | 13 | - |
| Compatibility page | 4 | 16 | 2 |
| Region A | 5 | 17 | 3 |
| Region B | 6 | 18 | 4 |
| Region C | 7 | 19 | 5 |

The Manager transcript remains 30 calls and 60 Agent/kernel address-space
switches. Its code length and return offsets remain stable while the embedded
IDs, result, and digest change.

The reference boot finishes with seven Resources, nineteen Capabilities, five
MemoryCells, and 205 Events. The five added events are Resource creation,
Capability grant, Capability derivation, MemoryCell creation, and fault-time
Resource retirement.

## QEMU Evidence

Strict debug and release validation requires:

- two authenticated Fault Worker Agent Calls before the first `#UD`;
- exact two-page mapping identity and proof values;
- `ResourceRetired` immediately before the first `TaskFaulted` for Agent 6;
- one terminal reclamation-log entry with Capability 8 and MemoryCell 1;
- successful restart after cleanup;
- all 16 runtime frames available and zero after the first fault and at final
  Manager completion;
- unchanged subsequent fault, handler, verifier, Manager, Driver, and handoff
  behavior;
- exactly 205 ordered events.

## Deferred Work

- cleanup after unrecoverable kernel-origin exceptions;
- asynchronous task cancellation with live runtime memory;
- cleanup of shared mappings with multiple owners;
- complete Agent address-space destruction;
- dynamic page-table intermediate allocation;
- SMP synchronization and TLB shootdown.
