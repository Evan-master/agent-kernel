# Agent Image Record Retirement V1 Design

## Status

Implemented and validated on 2026-07-19.

## Purpose

The fixed-capacity Agent Image Store retains every registered image record after
its lifecycle reaches `AgentImageStatus::Retired`. Long-running kernels can
therefore exhaust image metadata capacity even after executable identities have
become terminal and all semantic consumers have released them.

Agent Image Record Retirement V1 adds an authenticated cleanup endpoint for one
terminal image identity. It validates complete semantic and architecture-level
quiescence, removes the dense record atomically, emits replayable evidence, and
returns the physical slot to the Image Store while preserving monotonic IDs.

## Eligibility And Authority

`retire_agent_image_record(actor, authority, image)` requires:

1. an active registered actor;
2. an existing image in `AgentImageStatus::Retired`;
3. an active `Rollback` Capability held by the actor;
4. exact Resource scope while the image Resource is active, or ancestor scope
   while the image Resource is retired;
5. no Agent Entry reference to the image;
6. no Runtime Admission reference to the image;
7. one available Event slot.

The Core applies the shared terminal-metadata cleanup authorization contract.
This allows an administrator to clean terminal records under a retired Resource
tree without reviving or mutating that tree.

The x86 adapter adds one architecture preflight: no parked or resumable native
runtime context may carry the target Image ID.

## Strict Reference Preflight

Historical Events retain image identity and metadata as audit evidence and do
not keep the record resident. Every non-Event Core record that stores an
`AgentImageId` is authoritative liveness state.

Retirement rejects the target while referenced by:

- any Agent Entry, including terminal Agents whose entry record has not yet
  completed its own retirement lifecycle;
- any Runtime Admission record in Requested, Admitted, Rejected, or Released
  state.

The architecture adapter rejects any matching native Agent Call context before
invoking the Core. This prevents semantic deletion while executable state still
exists outside the Core arrays.

## Identity And Dense Removal

Image IDs already come from the monotonic `next_agent_image` allocator. Dense
removal never decrements or rewrites this allocator, so a fresh registration
cannot alias the retired identity. No additional tombstone or retirement floor
is required.

The operation performs a read-only preflight before mutation:

1. validate actor lifecycle and locate the target;
2. copy the complete target record;
3. require `Retired` status;
4. validate cleanup authority against the image Resource;
5. reject every non-Event reference;
6. reserve one Event slot.

After preflight, the suffix shifts left in stable order, the old tail resets to
`AgentImageRecord::empty()`, and `agent_image_len` decreases once. One Event is
then appended. Every failure preserves image records, order, allocator state,
and Event length.

## Receipt And Event

`AgentImageRecordRetirement` contains:

- the complete removed `AgentImageRecord`;
- the administrative actor;
- the authorizing Capability.

`AgentImageRecordRetired` records:

- the administrative actor in `agent`;
- the image owner in `target_agent`;
- the image Resource and cleanup Capability;
- `Operation::Rollback`;
- the retired Image ID, kind, digest, ABI version, and entry version.

This complete Event payload keeps replay and audit interpretation independent
from the removed Store record.

## Facade Contract

`AgentKernel::sys_retire_agent_image_record(actor, authority, image)` exposes
the Core result unchanged.

## Agent Call 37

The native ABI adds:

```text
RetireAgentImageRecord = 37
```

Request payload:

| Register | Value |
| --- | --- |
| `r10` | Rollback authority Capability ID |
| `r11` | retired target Agent Image ID |
| `r12-r15`, `rbp` | zero |

The scheduler-authenticated Agent, Task, Image, and nonce remain in `rsi`,
`rdi`, `r8`, and `r9`.

Successful reply payload:

| Register | Value |
| --- | --- |
| `r10` | retired Agent Image ID |
| `r11` | image Resource ID |
| `r12` | image owner Agent ID |
| `r13-r15`, `rbp` | zero |

The executor validates native runtime absence, complete receipt metadata, dense
removal, exact Event evidence, and unchanged running caller context before
encoding the canonical reply.

## X86 Capacity Proof

Bootstrap setup registers disposable Worker Image 9 on the bootstrap Resource
and retires its lifecycle before Resource Manager execution. Resource Manager
Capability 12 gains `Rollback` in addition to `Act` and `Delegate`.

The Resource Manager Capsule invokes Agent Call 37 for Image 9. The remaining
boot flow registers Images 10 through 14 while preserving the existing runtime
admission proof. After all native contexts and admission records are released,
bootstrap registers fresh Image 15 into the reclaimed physical slot.

The final Store contains 14 records with IDs 1 through 8 and 10 through 15.
This fills the configured capacity, proves physical slot reuse, and proves that
identity allocation stayed monotonic across record deletion.

## Failure Rules

- unknown actor or image: existing lookup error;
- suspended or retired actor: existing Agent lifecycle error;
- Pending or Verified image: `AgentImageRecordRetirementNotReady`;
- missing, foreign, task-scoped, revoked, attenuated, wrong-Resource, or
  wrong-operation authority: existing Capability or Resource error;
- any Agent Entry or Runtime Admission reference:
  `AgentImageRecordRetirementReferenced`;
- native runtime reference: Agent Call execution failure with no Core mutation;
- full Event Log: `EventLogFull` before Store mutation.

## Frozen Evidence

- Core and facade contracts cover terminal eligibility, cleanup authority,
  strict references, atomic failure, stable dense order, monotonic IDs, and
  physical capacity reuse.
- Agent Call 37 contracts cover decoding, reserved registers, authentication,
  native execution-context liveness, receipt validation, and canonical reply
  encoding.
- The Resource Manager Capsule executes 34 Agent Calls and 68 address-space
  switches. Its 3,195-byte artifact has SHA-256
  `d86e0918da3eb102ba24d382812c60cf005829888b508817bbd51ea34925af9e`.
- Independent assembly reproduces the 3,163-byte machine-code body exactly.
  The generated Rust Capsule matches the independently constructed header and
  body byte for byte.
- The final Image Store contains IDs 1 through 8 and 10 through 15. Image 9 is
  absent, Image 15 occupies the returned physical slot, and the Store remains
  at its configured 14-record capacity.
- The debug and release QEMU profiles both validate the exact 370-Event
  transcript. `AgentImageRecordRetired` is Event 179, the fresh Image
  registration is Event 360, and Driver completion is Event 370.
- The complete Resource Manager and Admission Supervisor Capsules each occur
  exactly once in the release ELF.

## Deferred Work

- retirement permits spanning semantic and architecture reclamation;
- executable-byte cache ownership and loader-cache invalidation;
- bounded retirement for Resources, Actions, Observations, Checkpoints, Memory
  Cells, Faults, Waiters, and Driver records;
- durable Event archival and replay checkpoints.
