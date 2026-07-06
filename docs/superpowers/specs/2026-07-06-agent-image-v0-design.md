# Agent Image / Executable Object V0 Design

## Purpose

Agent Image V0 gives the native kernel a first-class executable identity. The
kernel already knows that an agent has been launched and admitted into runtime
work, but it does not yet know what executable object that launch represents.
V0 closes that gap by adding fixed-capacity agent image records and binding each
launch entry to an active image.

This is not a POSIX process model, ELF loader, filesystem integration, package
manager, or code execution engine. It is the native AgentOS identity layer that
future loaders, address spaces, verification policies, and relaunch lifecycle
rules can build on.

## Scope

V0 provides:

- `AgentImageId` typed identifiers,
- `AgentImageKind::{Bootstrap, Supervisor, Worker}`,
- `AgentImageRecord` fixed-capacity store entries,
- image registration and retirement transitions,
- launch-time image validation for resource-scoped and task-scoped launches,
- image references on launch entries and launch events,
- facade syscalls and supervisor output for image registration and launch,
- deterministic tests for success, authority failure, capacity failure,
  retired-image rejection, resource mismatch, and task launch image mismatch.

V0 does not provide:

- byte storage for executable contents,
- ELF parsing or binary loading,
- host filesystem reads,
- dynamic linking,
- page tables or address spaces,
- running code inside the kernel,
- cryptographic hash computation in the kernel.

The supervisor supplies a fixed digest. The kernel stores and compares it but
does not compute it.

## Core Model

Add an executable identity record:

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

Add compact supporting types:

```rust
pub struct AgentImageDigest {
    pub bytes: [u8; 32],
}

pub enum AgentImageKind {
    Bootstrap,
    Supervisor,
    Worker,
}

pub enum AgentImageStatus {
    Active,
    Retired,
}
```

`AgentImageId` follows the existing typed-id pattern in `id.rs`.

The `KernelCore` image store is fixed-capacity:

```rust
agent_images: [AgentImageRecord; AGENT_IMAGES]
agent_image_len: usize
next_agent_image: u64
```

`AGENT_IMAGES` should be appended as a defaulted const generic after existing
stores so existing tests and type aliases do not need churn unless they exercise
image behavior.

## Registration Contract

Add:

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

The operation validates:

- owner is active,
- resource exists and is active,
- capability authorizes root `Operation::Act` for the resource,
- `abi_version != 0`,
- `entry_version != 0`,
- at least one image slot is available,
- at least one event slot is available.

On success, it stores an active image, allocates the next `AgentImageId`, and
records `EventKind::AgentImageRegistered`.

The image resource is the resource namespace this image is allowed to launch
within. For V0, one image belongs to exactly one resource.

## Retirement Contract

Add:

```rust
retire_agent_image(
    owner: AgentId,
    capability: CapabilityId,
    image: AgentImageId,
) -> Result<Event, KernelError>
```

The operation validates:

- owner is active,
- image exists,
- image is active,
- owner matches the image owner,
- capability authorizes root `Operation::Rollback` for the image resource,
- at least one event slot is available.

On success, it marks the image retired and records
`EventKind::AgentImageRetired`.

Retiring an image does not mutate existing launch entries. Existing running
entries remain audit-visible and runtime-admitted by their launch capability.
The retirement only blocks future launches. Launch-entry retirement is a
separate lifecycle feature.

## Launch Binding

Extend `AgentEntryRecord`:

```rust
pub struct AgentEntryRecord {
    pub agent: AgentId,
    pub resource: ResourceId,
    pub capability: CapabilityId,
    pub image: AgentImageId,
    pub kind: AgentEntryKind,
    pub intent: Option<IntentId>,
    pub task: Option<TaskId>,
}
```

Change launch APIs to require an image:

```rust
launch_agent(
    agent: AgentId,
    capability: CapabilityId,
    resource: ResourceId,
    image: AgentImageId,
    kind: AgentEntryKind,
    intent: Option<IntentId>,
) -> Result<Event, KernelError>

launch_task_agent(
    agent: AgentId,
    capability: CapabilityId,
    task: TaskId,
    image: AgentImageId,
    kind: AgentEntryKind,
) -> Result<Event, KernelError>
```

Existing launch validation still applies: active agent, active resource or
task, capability authority, duplicate-entry rejection, entry capacity, and event
capacity. V0 adds image validation before any mutation:

- image exists,
- image is active,
- image resource equals launch resource for resource-scoped launch,
- image resource equals task resource for task-scoped launch,
- image kind is compatible with the requested entry kind.

Compatibility is direct:

- `AgentImageKind::Bootstrap` can launch `AgentEntryKind::Bootstrap`,
- `AgentImageKind::Supervisor` can launch `AgentEntryKind::Supervisor`,
- `AgentImageKind::Worker` can launch `AgentEntryKind::Worker`.

V0 rejects cross-kind launches with `KernelError::AgentImageKindMismatch`.

## Event Model

Add event kinds:

- `AgentImageRegistered`,
- `AgentImageRetired`.

Extend `Event` with:

```rust
pub agent_image: Option<AgentImageId>
pub agent_image_kind: Option<AgentImageKind>
pub agent_image_digest: Option<AgentImageDigest>
pub agent_image_abi_version: Option<u16>
pub agent_image_entry_version: Option<u16>
```

`AgentImageRegistered` stores `agent_image`, `agent_image_kind`,
`agent_image_digest`, `agent_image_abi_version`,
`agent_image_entry_version`, `resource`, and the authorizing `capability`.
That is enough for event-log replay to reconstruct the image record.

`AgentImageRetired` stores `agent_image`, `agent_image_kind`, `resource`, and
the authorizing `capability`. It does not repeat digest or version fields
because replay can resolve them from the prior registration event.

`AgentLaunched` events store `agent_image` in addition to the existing
agent/resource/capability/intent/task fields.
Launch events do not repeat digest or version fields. They reference the active
image identity that registration already made replayable.

## Errors

Add explicit errors:

- `AgentImageNotFound`,
- `AgentImageStoreFull`,
- `AgentImageRetired`,
- `AgentImageStatusMismatch`,
- `AgentImageKindMismatch`,
- `AgentImageResourceMismatch`,
- `AgentImageVersionInvalid`.

Existing errors still apply for agent state, resource lookup, resource status,
capability authority, event capacity, duplicate launch entry, task mismatch,
and task status mismatch.

All validation failures occur before mutation. Successful registration,
retirement, and launch record events. Failed image checks are invisible because
no kernel state changed.

## Facade

Expose syscall-style wrappers in `agent-kernel`:

```rust
sys_register_agent_image(...)
sys_retire_agent_image(...)
agent_images() -> &[AgentImageRecord]
agent_image(image: AgentImageId) -> Result<AgentImageRecord, KernelError>
```

Update `sys_launch_agent` and `sys_launch_task_agent` to accept an
`AgentImageId`.

## Supervisor Flow

The supervisor registers:

- a supervisor image for agent 1,
- a worker image for agent 2.

Then:

- owner resource-scoped launch uses the supervisor image,
- delegated task-scoped launch uses the worker image,
- output includes image registration events before launch,
- `agent_launched` output includes `image=N`.

The boot flow registers a bootstrap image before launching the bootstrap agent
and prints an `agent_image_registered` event before `agent_launched`.

## Test Evidence

Core tests must prove:

- registering an image stores metadata and records an event,
- registration events carry digest and version fields needed for replay,
- registering requires root `Act` authority,
- image capacity failure leaves no event or record,
- retiring an image marks it retired and records an event,
- retired image cannot be launched,
- resource-scoped launch records the image on entry and event,
- task-scoped launch records the image on entry and event,
- image resource mismatch rejects launch without mutation,
- image kind mismatch rejects launch without mutation,
- invalid image versions are rejected without mutation.

Facade tests must prove:

- image registration and query work through syscall wrappers,
- launching with an image works for resource-scoped and task-scoped launch.

Supervisor tests must prove:

- image registration appears in output,
- launch output includes the expected image id.

Boot tests must prove:

- bootstrap image registration occurs before bootstrap launch,
- QEMU serial output includes the image registration event.

## Compatibility And Migration

This intentionally changes launch APIs. Existing tests and supervisor flows
must register images before launch. That is correct: after V0, a launch without
an executable identity is not a valid native AgentOS launch.

Existing kernel concepts remain unchanged:

- capabilities still authorize resource access,
- runtime admission still checks launch entries and live capabilities,
- task-scoped launch remains least-authority worker admission,
- executable bytes and actual code execution remain outside this V0 scope.

## Follow-Up Features

Agent Image V0 sets up these later milestones:

- image verification policy,
- address spaces and memory regions,
- loader-owned executable byte storage,
- launch retirement and relaunch lifecycle,
- signed image manifests,
- supervisor-side image registry adapters.
