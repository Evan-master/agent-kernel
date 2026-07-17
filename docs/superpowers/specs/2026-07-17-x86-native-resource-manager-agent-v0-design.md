# X86 Native Resource Manager Agent V0 Design

## Status

Implemented and validated locally on 2026-07-17; publication is pending.

## Purpose

The native x86 proof currently admits Agents, delegates tasks, and creates all
kernel resources from bootstrap. That proves isolated execution but leaves a
central bootstrap-only control point. Resource Manager Agent V0 gives a real
ring-3 Agent the first capability-checked resource lifecycle protocol: it uses
an explicitly delegated parent-resource capability to create a child resource,
receives the new resource and root capability as bounded kernel handles, uses
that capability to retire the child, and completes with an auditable result.

The proof path is:

```text
bootstrap delegates Workspace Act authority
    -> ring-3 Resource Manager Agent
    -> CreateResource Agent Call
    -> kernel capability-chain validation
    -> ResourceCreated + CapabilityGranted
    -> resource/capability handles returned to ring 3
    -> RetireResource Agent Call with returned capability
    -> ResourceRetired
    -> TaskResult + TaskCompleted
```

## Layer Placement

- `agent-kernel-core` owns canonical operation-set encoding and the existing
  deterministic resource ownership/lifecycle rules.
- `agent-kernel` remains the no-std syscall facade used by the architecture
  adapter; no resource store is exposed directly.
- The host-testable x86 library owns Agent Call operations 10 and 11, strict
  register decoding, trusted-context authentication, and bounded replies.
- The bare-metal x86 adapter owns the immutable Supervisor Capsule, its private
  address space and CPU state, boot admission, dispatch, transcript validation,
  and terminal QEMU evidence.

## Agent Call ABI

Agent Call ABI v1 reserves operation 10 for `CreateResource` and operation 11
for `RetireResource`. Both retain the common authenticated identity payload:

```text
rsi AgentId
rdi TaskId
r8  AgentImageId
r9  non-zero dispatch nonce
```

`CreateResource` uses:

```text
r10 parent authority CapabilityId
r11 parent ResourceId
r12 ResourceKind wire code
r13 requested OperationSet bits
r14/r15/rbp reserved zero
```

The native kind codes are Workspace 1, Memory 2, Service 3, Network 4, and
Device 5. File and Process remain legacy-facing resource kinds and are not
admitted by this native V0 ABI. The requested operation set must be non-empty,
fit the canonical six-bit operation mask, and contain no unknown bits.

On success the reply returns the created `ResourceId` in `r10` and its root
`CapabilityId` in `r11`. All remaining result words are zero.

`RetireResource` uses:

```text
r10 ResourceId
r11 Rollback-authorizing CapabilityId
r12/r13/r14/r15/rbp reserved zero
```

Its success reply echoes the retired resource and authorizing capability in
`r10/r11`. Zero IDs, unknown kinds or operation bits, non-zero reserved fields,
stale dispatch identity, and stale nonce all fail closed before mutation.

## Authority Model

Bootstrap derives one resource-scoped capability from its Workspace root and
delegates it to the Resource Manager with only `Act`. The capability handle is
carried in the create request; it is not hidden in the scheduler context. Core
authorization validates owner, resource scope, ancestor revocation, and `Act`.

The child Service is created with `Observe | Act | Rollback`. Creation mints a
new root capability owned by the requesting Agent. Retirement then requires
that returned child capability and the existing core `Rollback` check. The
task-scoped launch capability cannot authorize either resource operation.

Every successful mutation retains the existing exact event consequences:

```text
ResourceCreated
CapabilityGranted
ResourceRetired
```

No architecture code mutates resource or capability stores directly.

## Native Capsule And Runtime Proof

Agent 8 runs a kind-4 Supervisor Capsule with its own page-table root, code,
signal, stack, and retained-page frames. Before its first Agent Call it waits
for physical quantum generation 1, forcing one PIT expiry and redispatch. It
then makes exactly five calls:

1. DescribeContext.
2. CreateResource under Workspace 1 using the delegated authority.
3. RetireResource using the returned child capability.
4. SubmitTaskResult with the resource/capability pair.
5. CompleteTask.

The Capsule accepts only the expected success operation, `ResourceId 2`, and
`CapabilityId 11`. Its result code is `0xc001`; value
`0x000000020000000b` packs the resource ID in the high 32 bits and capability
ID in the low 32 bits. Boot validates the exact call transcript, returned
handles, records, authority parent chain, event suffix, task result, terminal
CPU state, private memory ownership, and empty runtime queue.

The canonical kind-4 Capsule is 237 bytes: a 32-byte header and 205 bytes of
code generated from the adjacent `resource_manager.S`. It uses nonce
`0xf66ce006`, return offsets `45/86/132/194/203`, and SHA-256 digest
`9d8a7fbe103c43d16a810737424b6c2f123086436952cb2113dc1f3579da241c`.

## Capacity And Event Proof

The deterministic boot proof contains eight Agents, two resources, eleven
capabilities, six tasks/intents/completed native contexts, and 169 events.
Resource Manager setup contributes eleven events. Its runtime contributes two
dispatches, one physical quantum expiry, resource creation/grant/retirement,
result submission, and completion. The final proof therefore requires 23
dispatches, 10 preemptions, 10 physical quantum expiries, and six native
completions while retaining the existing fault, mailbox, and Driver evidence.

Manager setup is events 71 through 81. Existing native execution shifts to
events 82 through 150. The Manager's queue, two dispatches, physical expiry,
create/grant/retire, result, and completion occupy events 151 through 159. The
Driver flow remains terminal at events 160 through 169.

The terminal marker is:

```text
AGENT_KERNEL_NATIVE_RESOURCE_MANAGER_AGENT_OK
```

## Failure Policy

Boot fails closed on ABI mismatch, invalid operation bits, unsupported resource
kind, capability denial, partial event emission, unexpected deterministic ID,
Capsule transcript mismatch, resource record mismatch, missing retirement,
memory aliasing, counter drift, or non-empty scheduler/runtime state.

## Validation

- Red/green host tests cover canonical operation-set encoding, both Agent Call
  request/reply contracts, context authentication, and malformed payloads.
- Core and facade suites retain atomic authorization and capacity coverage.
- Formatting, full workspace tests, Supervisor execution, no-std checks, and
  warnings-denied scoped Clippy pass.
- Debug and release QEMU prove the new marker and exactly 169 events.
- Release ELF inspection reproduces the Capsule digest, and disassembly shows
  Agent Call operations `1 -> 10 -> 11 -> 4 -> 3` at the five `int 0x90`
  boundaries.
- `README.md` and `README.zh-CN.md` provide synchronized English and Simplified
  Chinese project entry points.

## Non-Goals

V0 does not provide arbitrary resource enumeration, capability transfer between
unrelated Agents, resource reopening, shared ownership, asynchronous resource
brokers, dynamic memory allocation, arbitrary page-table control, host files or
processes, network I/O, SMP synchronization, or a general userspace driver
model. Those require later native protocols built on this lifecycle boundary.
