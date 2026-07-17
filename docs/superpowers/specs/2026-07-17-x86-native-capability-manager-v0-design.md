# X86 Native Capability Manager V0 Design

## Status

Implemented and validated locally in debug and release QEMU on 2026-07-17;
publication is pending.

## Purpose

Resource Manager V0 lets a real ring-3 Agent create and retire one child
resource, but only bootstrap can still move authority between Agents. Native
Capability Manager V0 closes that control point. The Resource Manager derives
an attenuated capability for another registered Agent, receives the new kernel
handle, revokes that exact child through its source capability, and proves the
entire lifecycle through ordered events.

The proof path is:

```text
ring-3 Resource Manager creates child Service with Delegate
    -> DeriveCapability Agent Call
    -> source owner, scope, chain, and operation checks
    -> CapabilityDerived for another Agent
    -> derived CapabilityId returned to ring 3
    -> RevokeDerivedCapability Agent Call
    -> direct-parent and source authority checks
    -> CapabilityRevoked
    -> child Service retirement and task completion
```

## Layer Placement

- `agent-kernel-core` owns authenticated direct-child revocation and its atomic
  event consequence.
- `agent-kernel` exposes that operation through the no-std syscall facade.
- The host-testable x86 ABI owns Agent Call operations 12 and 13, strict
  register decoding, context authentication, and canonical replies.
- The bare-metal x86 adapter invokes only facade methods and validates the
  exact records and event suffix.
- The existing kind-4 Resource Manager Capsule performs the protocol from its
  private ring-3 address space.

## Core Authority Contract

The new facade operation is:

```text
sys_revoke_derived_capability(actor, source, target)
```

It succeeds only when:

1. `actor` is active.
2. `source` exists, belongs to `actor`, has an active ancestor chain, is not
   Task-scoped, and includes `Delegate`.
3. `target` exists on the same Resource and has `parent == Some(source)`.
4. `target` is not already revoked.
5. one event slot is available before mutation.

Success marks only `target` revoked and emits one `CapabilityRevoked` event.
The event records `actor`, `target`, `source_capability`, the target operation
set, and the target Agent. V0 intentionally requires a direct parent instead
of walking an arbitrary descendant tree.

The existing trusted kernel maintenance revocation primitive remains outside
the ring-3 ABI. Architecture code must not call it for this flow.

## Agent Call ABI

Both calls retain the common authenticated identity payload:

```text
rsi AgentId
rdi TaskId
r8  AgentImageId
r9  non-zero dispatch nonce
```

Agent Call ABI v1 adds operation 12, `DeriveCapability`:

```text
r10 source CapabilityId
r11 target AgentId
r12 requested OperationSet bits
r13/r14/r15/rbp reserved zero
```

The requested set must be non-empty and use only the canonical six operation
bits. Core authorization additionally requires it to be a subset of the source
and requires the source to include `Delegate`. Success returns the derived
`CapabilityId` in `r10`; all other result words are zero.

Operation 13, `RevokeDerivedCapability`, uses:

```text
r10 source CapabilityId
r11 target CapabilityId
r12/r13/r14/r15/rbp reserved zero
```

Success echoes the revoked target in `r10` and the authorizing source in
`r11`. Zero handles, unknown bits, non-zero reserved fields, stale identity,
and stale nonce fail closed before mutation.

## Native Capsule Proof

The Resource Manager creates Service Resource 2 with
`Observe | Act | Delegate | Rollback`, receiving Capability 11. It then:

1. derives `Observe` as Capability 12 for Agent 2;
2. revokes Capability 12 using Capability 11;
3. retires Resource 2 using Capability 11;
4. submits result code `0xc002` with a value packing Resource 2, source
   Capability 11, and derived Capability 12;
5. completes its Task.

Together with `DescribeContext` and `CreateResource`, the immutable Capsule
makes exactly seven Agent Calls. The existing physical PIT expiry before the
first call remains required, so scheduling and preemption counts do not change.

The resulting kind-4 Capsule is 399 bytes: a 32-byte header and 367 bytes of
code generated from `resource_manager.S`. It retains nonce `0xf66ce006`, uses
return offsets `45/86/163/236/294/356/365`, and has SHA-256 digest
`29d6f533b3e959de9cf63b662e5295ca9e007dcc15a922b665edb6c024b328da`.

Terminal evidence validates the exact call transcript, returned handles,
source/target records, direct parent, revoked state, event fields, final task
result, CPU state, and private memory ownership.

## Capacity And Event Projection

The proof remains at eight Agents, two Resources, six native completions,
twenty-three dispatches, and ten physical quantum expiries. Capability capacity
and final count increase from eleven to twelve.

The Manager runtime gains `CapabilityDerived` and `CapabilityRevoked` between
resource creation and retirement. Its runtime suffix grows from nine to eleven
events. Driver events shift by two, and the deterministic terminal event count
increases from 169 to 171.

Manager setup remains events 71 through 81. Existing native execution remains
events 82 through 150. Manager runtime occupies events 151 through 161, with
`CapabilityDerived` at 157 and `CapabilityRevoked` at 158. Driver execution is
terminal at events 162 through 171.

The dedicated terminal marker is:

```text
AGENT_KERNEL_NATIVE_CAPABILITY_MANAGER_OK
```

## Failure Policy

The proof fails closed on source ownership mismatch, missing `Delegate`, Task
scope, non-subset operations, wrong direct parent, cross-resource target,
already-revoked target, partial event emission, unexpected IDs, transcript
mismatch, record mismatch, counter drift, or any malformed ABI payload.

## Validation

- Core tests cover success, source ownership, operation, scope, lineage,
  suspended recipients, already-revoked targets, and event-capacity atomicity.
- Facade tests prove no architecture-only mutation path is needed.
- ABI tests cover decode, trusted-context authentication, canonical replies,
  zero handles, unknown operation bits, and reserved registers.
- Formatting, the full workspace, Supervisor, no-std checks, warnings-denied
  scoped Clippy, and bare-metal Clippy pass.
- Debug and release QEMU emit the dedicated marker and exactly 171 events.
- Release ELF inspection reproduces the 399-byte Capsule digest. Disassembly
  shows Agent Call operations `1 -> 10 -> 12 -> 13 -> 11 -> 4 -> 3` at the
  seven `int 0x90` boundaries.

## Non-Goals

V0 does not enumerate capabilities, revoke arbitrary descendants, transfer
ownership, create unregistered Agents, expose bootstrap grants, revoke a source
root from ring 3, dynamically load images, or launch new Agents. Those remain
separate native protocols.
