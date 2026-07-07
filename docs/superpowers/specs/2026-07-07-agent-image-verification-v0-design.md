# Agent Image Verification V0 Design

## Purpose

Agent Image Verification V0 turns executable launchability into a
kernel-visible lifecycle gate. Agent Image V0 gave the kernel a fixed executable
identity, but a registered image is currently launchable immediately. That makes
registration and approval the same event, which is too weak for an agent-native
kernel where verification should be explicit, replayable, and authorized.

This design separates image registration from image verification:

```text
Pending -> Verified -> Retired
```

Only verified images may be launched. Registration records that an executable
identity exists. Verification records that the kernel accepted that identity for
future launches under explicit authority.

## Scope

V0 provides:

- `AgentImageStatus::{Pending, Verified, Retired}`,
- image registration that creates `Pending` records,
- `verify_agent_image(owner, capability, image)`,
- `EventKind::AgentImageVerified`,
- launch checks that reject unverified images,
- facade syscall `sys_verify_agent_image`,
- boot and supervisor flows that verify images before launch,
- QEMU and README trace updates for `agent_image_verified`,
- tests for the verification transition, launch gating, authority failures,
  owner failures, status failures, event-capacity failures, and retirement.

V0 does not provide:

- executable byte storage,
- ELF parsing,
- binary loading,
- address spaces,
- page tables,
- hash computation in the kernel,
- signature checking,
- verifier plugins,
- host filesystem access.

The supervisor still supplies fixed digest metadata. The kernel stores and
audits lifecycle decisions; it does not inspect executable bytes in V0.

## Core Model

Change `AgentImageStatus` from:

```rust
pub enum AgentImageStatus {
    Active,
    Retired,
}
```

to:

```rust
pub enum AgentImageStatus {
    Pending,
    Verified,
    Retired,
}
```

`Pending` means the image identity is registered but not launchable.
`Verified` means the image passed the kernel-visible approval gate and can be
used by launch APIs if the existing resource and kind checks also pass.
`Retired` means the image can no longer transition or launch.

`AgentImageRecord` keeps its existing fields:

```rust
pub struct AgentImageRecord {
    pub id: AgentImageId,
    pub owner: AgentId,
    pub resource: ResourceId,
    pub kind: AgentImageKind,
    pub digest: AgentImageDigest,
    pub abi_version: u16,
    pub entry_version: u16,
    pub status: AgentImageStatus,
}
```

No new store is added. Verification mutates the existing fixed-capacity image
store and records an event.

## Registration Contract

`register_agent_image` keeps its existing signature and validation:

```rust
register_agent_image(
    owner: AgentId,
    capability: CapabilityId,
    resource: ResourceId,
    kind: AgentImageKind,
    digest: AgentImageDigest,
    abi_version: u16,
    entry_version: u16,
) -> Result<AgentImageId, KernelError>
```

The success result changes from storing an active image to storing a pending
image. It still records `EventKind::AgentImageRegistered`.

Registration does not require `Operation::Verify`. It keeps the current
`Operation::Act` authority because registration declares an executable identity
for a resource namespace. Verification is the separate approval gate.

Failed registration remains invisible because no state changes.

## Verification Contract

Add:

```rust
verify_agent_image(
    owner: AgentId,
    capability: CapabilityId,
    image: AgentImageId,
) -> Result<Event, KernelError>
```

The operation validates, before mutation:

- owner is active,
- image exists,
- image owner matches `owner`,
- image status is `Pending`,
- capability authorizes `Operation::Verify` for the image resource,
- one event slot is available.

On success, the kernel:

- changes the image status from `Pending` to `Verified`,
- records `EventKind::AgentImageVerified`,
- returns the verification event.

`AgentImageVerified` stores:

- `agent`,
- `resource`,
- `capability`,
- `agent_image`,
- `agent_image_kind`.

It does not repeat digest or version fields because replay can resolve them from
the prior `AgentImageRegistered` event. Verification is a lifecycle decision
over a previously registered image, not a new executable identity.

Verifying an already verified image returns
`KernelError::AgentImageStatusMismatch`. Verifying a retired image returns
`KernelError::AgentImageRetired`.

## Launch Binding

`launch_agent` and `launch_task_agent` keep their current image argument. Their
image validation changes from "image is active" to "image is verified".

Resource-scoped launch accepts an image only when:

- image exists,
- image status is `Verified`,
- image resource equals the launch resource,
- image kind matches the requested `AgentEntryKind`.

Task-scoped launch accepts an image only when:

- image exists,
- image status is `Verified`,
- image resource equals the task resource,
- image kind matches the requested `AgentEntryKind`.

Pending images fail with `KernelError::AgentImageStatusMismatch`. Retired images
continue to fail with `KernelError::AgentImageRetired`. Kind and resource
mismatches keep the existing `AgentImageKindMismatch` and
`AgentImageResourceMismatch` errors.

Launch failures remain invisible because no launch entry or event is written.

## Retirement Contract

`retire_agent_image` keeps its existing signature:

```rust
retire_agent_image(
    owner: AgentId,
    capability: CapabilityId,
    image: AgentImageId,
) -> Result<Event, KernelError>
```

It may retire `Pending` or `Verified` images. Retiring an already retired image
returns `KernelError::AgentImageRetired`.

Successful retirement still:

- requires owner match,
- requires rollback authority over the image resource,
- records `EventKind::AgentImageRetired`,
- blocks future launches.

Retirement does not mutate existing launch entries. Existing launch entries
remain audit-visible as historical runtime admissions.

## Event Model

Add:

```rust
AgentImageVerified
```

to `EventKind`.

The event field set is:

```text
agent = owner
kind = AgentImageVerified
resource = Some(image.resource)
capability = Some(capability)
target_agent = None
agent_image = Some(image)
agent_image_kind = Some(image.kind)
agent_image_digest = None
agent_image_abi_version = None
agent_image_entry_version = None
```

Replay reconstructs image state as:

- `AgentImageRegistered` creates a `Pending` image,
- `AgentImageVerified` changes it to `Verified`,
- `AgentImageRetired` changes it to `Retired`.

Every successful lifecycle mutation records exactly one event.

## Errors And Atomicity

Use existing errors:

- `AgentNotFound`, `AgentSuspended`, or `AgentRetired` from active-owner
  validation,
- `AgentImageNotFound`,
- `AgentMismatch`,
- `AgentImageStatusMismatch`,
- `AgentImageRetired`,
- `OperationDenied`,
- `CapabilityNotFound`,
- `CapabilityRevoked`,
- `CapabilityScopeMismatch`,
- `EventLogFull`.

No new error is required for V0.

All verification validation happens before mutation. If the event log is full,
the image remains `Pending`. If authority, owner, image lookup, or status checks
fail, the image record and event log are unchanged.

## Facade

Add to `agent-kernel`:

```rust
sys_verify_agent_image(
    owner: AgentId,
    capability: CapabilityId,
    image: AgentImageId,
) -> Result<Event, KernelError>
```

The facade delegates to `KernelCore::verify_agent_image` and does not expose a
shortcut around core authorization or state checks.

Existing `agent_images()` and `agent_image(image)` inspection APIs remain enough
to observe `Pending`, `Verified`, and `Retired` states.

## Supervisor, Boot, And QEMU

Host-side flows must verify images before launch:

- boot registers the bootstrap image, verifies it, then launches it,
- supervisor registers the supervisor image, verifies it, then launches it,
- task-worker examples register worker images, verify them, then launch them.

Serial and supervisor output should include `agent_image_verified` so a minimal
run shows this sequence:

```text
agent_image_registered
agent_image_verified
agent_launched
```

The QEMU trace should include the new event between registration and launch.

## Tests

Core tests must prove:

- registering an image stores `AgentImageStatus::Pending`,
- verifying a pending image changes status to `Verified`,
- verification records `EventKind::AgentImageVerified`,
- an unverified image cannot launch,
- a verified image can launch,
- task-scoped launch also requires a verified image,
- verification rejects a missing image without mutation,
- verification rejects a non-owner without mutation,
- verification rejects missing or wrong `Operation::Verify` authority without
  mutation,
- verification rejects a verified image with `AgentImageStatusMismatch`,
- verification rejects a retired image with `AgentImageRetired`,
- event-log-full verification leaves the image `Pending`,
- retiring a pending image succeeds and blocks launch,
- retiring a verified image succeeds and blocks launch.

Facade tests must prove:

- `sys_verify_agent_image` exposes the verified status and event,
- `sys_launch_agent` rejects pending images and accepts verified images.

Boot, supervisor, and QEMU validation must prove the deterministic trace includes
image registration, image verification, and launch in that order.

## Compatibility Impact

This is an intentional breaking behavior change for in-repo callers:
registration no longer makes an image launchable. Every caller that launches an
image must hold or obtain verify authority and call `verify_agent_image` first.

The public image registration and launch signatures remain stable. The new
syscall is additive, and the behavior change is visible through tests and
README traces.

## Future Work

Future verifier layers may attach real evidence to this lifecycle gate:

- signature verification,
- byte digest computation,
- loader validation,
- ABI compatibility checks,
- verifier capability classes,
- image provenance records,
- revocation effects on already launched entries.

Those features build on the same `Pending -> Verified -> Retired` lifecycle but
stay out of V0 to keep the kernel deterministic, no_std-friendly, and testable.
