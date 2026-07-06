# Agent Image V0 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add first-class, replayable executable identity records and require every agent launch to bind to an active Agent Image.

**Architecture:** `agent-kernel-core` owns fixed-capacity image records, image registration/retirement, launch validation, and replayable event fields. `agent-kernel` exposes syscall-style wrappers. `agent-supervisor`, `agent-kernel-boot`, `agent-kernel-x86_64`, tests, and README migrate to register images before launch and render image provenance.

**Tech Stack:** Rust nightly, no_std core and facade crates, fixed-capacity arrays, typed IDs, Cargo workspace tests, QEMU BIOS boot script.

---

## File Structure

- Create `crates/agent-kernel-core/src/agent_image.rs` for `AgentImageRecord`, `AgentImageDigest`, `AgentImageKind`, and `AgentImageStatus`.
- Create `crates/agent-kernel-core/src/agent_image_store.rs` for registration, retirement, lookup, and image validation helpers.
- Modify `crates/agent-kernel-core/src/id.rs` for `AgentImageId`.
- Modify `crates/agent-kernel-core/src/lib.rs` to wire modules and exports.
- Modify `crates/agent-kernel-core/src/core.rs` to add the fixed image store and defaulted `AGENT_IMAGES` const generic after existing generics.
- Modify `crates/agent-kernel-core/src/event.rs` for image event kinds and replay fields.
- Modify `crates/agent-kernel-core/src/error.rs` for explicit image errors.
- Modify `crates/agent-kernel-core/src/agent_entry.rs` to store `image: AgentImageId`.
- Modify `crates/agent-kernel-core/src/agent_launch.rs` to require and validate image identity before writing launch entries.
- Modify `crates/agent-kernel/src/lib.rs` and `crates/agent-kernel/src/agent.rs` to propagate the new generic and syscall wrappers.
- Modify supervisor, boot, x86 serial, README, and existing tests that launch agents.

## Task 1: Core Image Registration Red Tests

**Files:**
- Create: `crates/agent-kernel-core/tests/agent_image.rs`
- Later modify: `crates/agent-kernel-core/src/id.rs`
- Later modify: `crates/agent-kernel-core/src/agent_image.rs`
- Later modify: `crates/agent-kernel-core/src/agent_image_store.rs`
- Later modify: `crates/agent-kernel-core/src/core.rs`
- Later modify: `crates/agent-kernel-core/src/event.rs`
- Later modify: `crates/agent-kernel-core/src/error.rs`
- Later modify: `crates/agent-kernel-core/src/lib.rs`

- [x] **Step 1: Write failing registration tests**

Create `crates/agent-kernel-core/tests/agent_image.rs`:

```rust
use agent_kernel_core::{
    AgentId, AgentImageDigest, AgentImageKind, AgentImageStatus, EventKind, KernelCore,
    KernelError, Operation, OperationSet, ResourceKind,
};

type ImageCore = KernelCore<2, 2, 4, 16, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 2>;

fn digest(byte: u8) -> AgentImageDigest {
    AgentImageDigest::new([byte; 32])
}

fn prepare_owner(core: &mut ImageCore, operations: OperationSet) -> (AgentId, agent_kernel_core::CapabilityId, agent_kernel_core::ResourceId) {
    let owner = AgentId::new(1);
    core.register_agent(owner).expect("owner should register");
    let resource = core
        .register_resource(ResourceKind::Workspace, None)
        .expect("resource should fit");
    let capability = core
        .grant_capability(owner, resource, operations)
        .expect("capability should fit");
    (owner, capability, resource)
}

#[test]
fn register_agent_image_stores_metadata_and_replayable_event() {
    let mut core = ImageCore::new();
    let (owner, capability, resource) =
        prepare_owner(&mut core, OperationSet::only(Operation::Act));
    let image_digest = digest(7);

    let image = core
        .register_agent_image(
            owner,
            capability,
            resource,
            AgentImageKind::Supervisor,
            image_digest,
            1,
            2,
        )
        .expect("image should register");

    assert_eq!(image.raw(), 1);
    let image_record = core.agent_image(image).expect("image should be queryable");
    assert_eq!(image_record.id, image);
    assert_eq!(image_record.owner, owner);
    assert_eq!(image_record.resource, resource);
    assert_eq!(image_record.kind, AgentImageKind::Supervisor);
    assert_eq!(image_record.digest, image_digest);
    assert_eq!(image_record.abi_version, 1);
    assert_eq!(image_record.entry_version, 2);
    assert_eq!(image_record.status, AgentImageStatus::Active);

    let event = core.events().last().expect("registration should record event");
    assert_eq!(event.kind, EventKind::AgentImageRegistered);
    assert_eq!(event.agent, owner);
    assert_eq!(event.resource, Some(resource));
    assert_eq!(event.capability, Some(capability));
    assert_eq!(event.agent_image, Some(image));
    assert_eq!(event.agent_image_kind, Some(AgentImageKind::Supervisor));
    assert_eq!(event.agent_image_digest, Some(image_digest));
    assert_eq!(event.agent_image_abi_version, Some(1));
    assert_eq!(event.agent_image_entry_version, Some(2));
}

#[test]
fn register_agent_image_requires_act_authority_without_mutation() {
    let mut core = ImageCore::new();
    let (owner, capability, resource) =
        prepare_owner(&mut core, OperationSet::only(Operation::Observe));
    let events_before = core.events().len();

    let result = core.register_agent_image(
        owner,
        capability,
        resource,
        AgentImageKind::Worker,
        digest(3),
        1,
        1,
    );

    assert_eq!(result, Err(KernelError::OperationDenied));
    assert!(core.agent_images().is_empty());
    assert_eq!(core.events().len(), events_before);
}

#[test]
fn register_agent_image_rejects_zero_versions_without_mutation() {
    let mut core = ImageCore::new();
    let (owner, capability, resource) =
        prepare_owner(&mut core, OperationSet::only(Operation::Act));
    let events_before = core.events().len();

    let abi_result = core.register_agent_image(
        owner,
        capability,
        resource,
        AgentImageKind::Worker,
        digest(4),
        0,
        1,
    );
    let entry_result = core.register_agent_image(
        owner,
        capability,
        resource,
        AgentImageKind::Worker,
        digest(5),
        1,
        0,
    );

    assert_eq!(abi_result, Err(KernelError::AgentImageVersionInvalid));
    assert_eq!(entry_result, Err(KernelError::AgentImageVersionInvalid));
    assert!(core.agent_images().is_empty());
    assert_eq!(core.events().len(), events_before);
}

#[test]
fn register_agent_image_store_full_leaves_event_log_unchanged() {
    let mut core = KernelCore::<2, 2, 4, 16, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0>::new();
    let owner = AgentId::new(1);
    core.register_agent(owner).expect("owner should register");
    let resource = core
        .register_resource(ResourceKind::Workspace, None)
        .expect("resource should fit");
    let capability = core
        .grant_capability(owner, resource, OperationSet::only(Operation::Act))
        .expect("capability should fit");
    let events_before = core.events().len();

    let result = core.register_agent_image(
        owner,
        capability,
        resource,
        AgentImageKind::Worker,
        digest(6),
        1,
        1,
    );

    assert_eq!(result, Err(KernelError::AgentImageStoreFull));
    assert!(core.agent_images().is_empty());
    assert_eq!(core.events().len(), events_before);
}
```

- [x] **Step 2: Run the focused test and verify red**

Run:

```bash
PATH="$HOME/.cargo/bin:$PATH" RUSTC="$(rustup which rustc --toolchain nightly)" rustup run nightly cargo test -p agent-kernel-core --test agent_image
```

Expected: compile failure naming missing `AgentImageDigest`, `AgentImageKind`, `AgentImageStatus`, `register_agent_image`, `agent_image`, and `agent_images`.

## Task 2: Core Image Model And Registration

**Files:**
- Modify: `crates/agent-kernel-core/src/id.rs`
- Create: `crates/agent-kernel-core/src/agent_image.rs`
- Create: `crates/agent-kernel-core/src/agent_image_store.rs`
- Modify: `crates/agent-kernel-core/src/core.rs`
- Modify: `crates/agent-kernel-core/src/event.rs`
- Modify: `crates/agent-kernel-core/src/error.rs`
- Modify: `crates/agent-kernel-core/src/lib.rs`

- [x] **Step 1: Add typed image id and model**

Add `AgentImageId` to `id.rs` with the same `new` and `raw` shape as other ids.

Add `agent_image.rs`:

```rust
//! Kernel-owned agent executable identity records.
//!
//! This core-layer module defines fixed-width image metadata. It stores
//! provenance and compatibility identity only; executable bytes, loaders, and
//! hash computation stay outside the no_std kernel core.

use crate::{AgentId, AgentImageId, ResourceId};

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct AgentImageDigest {
    pub bytes: [u8; 32],
}

impl AgentImageDigest {
    pub const fn new(bytes: [u8; 32]) -> Self {
        Self { bytes }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum AgentImageKind {
    Bootstrap,
    Supervisor,
    Worker,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum AgentImageStatus {
    Active,
    Retired,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
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

impl AgentImageRecord {
    pub(crate) const fn empty() -> Self {
        Self {
            id: AgentImageId::new(0),
            owner: AgentId::new(0),
            resource: ResourceId::new(0),
            kind: AgentImageKind::Worker,
            digest: AgentImageDigest::new([0; 32]),
            abi_version: 0,
            entry_version: 0,
            status: AgentImageStatus::Retired,
        }
    }
}
```

- [x] **Step 2: Wire event, error, core store, and exports**

Update `event.rs` by inserting `AgentImageDigest`, `AgentImageId`, and
`AgentImageKind` into the current `use crate::{...}` import list.

Insert these variants immediately after `AgentRegistered`:

```rust
AgentImageRegistered,
AgentImageRetired,
```

Insert these fields after `target_agent`:

```rust
pub agent_image: Option<AgentImageId>,
pub agent_image_kind: Option<AgentImageKind>,
pub agent_image_digest: Option<AgentImageDigest>,
pub agent_image_abi_version: Option<u16>,
pub agent_image_entry_version: Option<u16>,
```

Set all new fields to `None` in `Event::empty()` and every `Event { ... }` literal found by:

```bash
rg -n "Event \\{" crates/agent-kernel-core/src
```

Update `error.rs` with:

```rust
AgentImageNotFound,
AgentImageStoreFull,
AgentImageRetired,
AgentImageStatusMismatch,
AgentImageKindMismatch,
AgentImageResourceMismatch,
AgentImageVersionInvalid,
```

Append `const AGENT_IMAGES: usize = 0` after `WAITERS` in `KernelCore` and add:

```rust
pub(crate) agent_images: [AgentImageRecord; AGENT_IMAGES],
pub(crate) agent_image_len: usize,
pub(crate) next_agent_image: u64,
```

Initialize with:

```rust
agent_images: [AgentImageRecord::empty(); AGENT_IMAGES],
agent_image_len: 0,
next_agent_image: 1,
```

Update every `KernelCore<...>` impl header in core modules to include the final `AGENT_IMAGES` generic. Existing call sites with fewer generics must keep compiling because the new generic is defaulted.

Update `lib.rs` with:

```rust
mod agent_image;
mod agent_image_store;

pub use agent_image::{AgentImageDigest, AgentImageKind, AgentImageRecord, AgentImageStatus};
```

Also add `AgentImageId` to the current public `pub use id::{...};` list.

- [x] **Step 3: Implement image registration**

Create `agent_image_store.rs` with registration, lookup, and event recording:

```rust
//! Fixed-capacity Agent Image store.
//!
//! This core-layer module owns image registration, retirement, and launch-time
//! validation. It stores executable identity metadata only and keeps all
//! mutation replayable through explicit image events.

use crate::{
    AgentId, AgentImageDigest, AgentImageId, AgentImageKind, AgentImageRecord, AgentImageStatus,
    CapabilityId, Event, EventKind, KernelCore, KernelError, Operation, OperationSet, ResourceId,
    VerificationRequirement,
};

impl<
        const AGENTS: usize,
        const RESOURCES: usize,
        const CAPS: usize,
        const EVENTS: usize,
        const ACTIONS: usize,
        const OBSERVATIONS: usize,
        const CHECKPOINTS: usize,
        const INTENTS: usize,
        const TASKS: usize,
        const RUN_QUEUE: usize,
        const MESSAGES: usize,
        const MEMORY_CELLS: usize,
        const NAMESPACE_ENTRIES: usize,
        const FAULTS: usize,
        const FAULT_HANDLERS: usize,
        const FAULT_POLICIES: usize,
        const WAITERS: usize,
        const AGENT_IMAGES: usize,
    >
    KernelCore<
        AGENTS,
        RESOURCES,
        CAPS,
        EVENTS,
        ACTIONS,
        OBSERVATIONS,
        CHECKPOINTS,
        INTENTS,
        TASKS,
        RUN_QUEUE,
        MESSAGES,
        MEMORY_CELLS,
        NAMESPACE_ENTRIES,
        FAULTS,
        FAULT_HANDLERS,
        FAULT_POLICIES,
        WAITERS,
        AGENT_IMAGES,
    >
{
    pub fn register_agent_image(
        &mut self,
        owner: AgentId,
        capability: CapabilityId,
        resource: ResourceId,
        kind: AgentImageKind,
        digest: AgentImageDigest,
        abi_version: u16,
        entry_version: u16,
    ) -> Result<AgentImageId, KernelError> {
        self.ensure_agent_active(owner)?;
        self.ensure_authorized(owner, capability, resource, Operation::Act)?;
        if abi_version == 0 || entry_version == 0 {
            return Err(KernelError::AgentImageVersionInvalid);
        }
        if self.agent_image_len >= AGENT_IMAGES {
            return Err(KernelError::AgentImageStoreFull);
        }
        self.ensure_event_slots(1)?;

        let image = AgentImageId::new(self.next_agent_image);
        self.next_agent_image += 1;
        self.agent_images[self.agent_image_len] = AgentImageRecord {
            id: image,
            owner,
            resource,
            kind,
            digest,
            abi_version,
            entry_version,
            status: AgentImageStatus::Active,
        };
        self.agent_image_len += 1;
        self.record_agent_image_registered_event(
            owner,
            capability,
            resource,
            image,
            kind,
            digest,
            abi_version,
            entry_version,
        )?;
        Ok(image)
    }

    pub fn agent_images(&self) -> &[AgentImageRecord] {
        &self.agent_images[..self.agent_image_len]
    }

    pub fn agent_image(&self, image: AgentImageId) -> Result<AgentImageRecord, KernelError> {
        self.find_agent_image(image)
    }

    pub(crate) fn find_agent_image(
        &self,
        image: AgentImageId,
    ) -> Result<AgentImageRecord, KernelError> {
        self.agent_images()
            .iter()
            .find(|record| record.id == image)
            .copied()
            .ok_or(KernelError::AgentImageNotFound)
    }
}
```

Use a private `record_agent_image_registered_event` helper that writes all image replay fields. Do not duplicate digest/version fields on non-registration events.

- [x] **Step 4: Verify registration green**

Run:

```bash
PATH="$HOME/.cargo/bin:$PATH" RUSTC="$(rustup which rustc --toolchain nightly)" rustup run nightly cargo test -p agent-kernel-core --test agent_image
```

Expected: all tests in `agent_image.rs` pass.

## Task 3: Image Retirement

**Files:**
- Modify: `crates/agent-kernel-core/tests/agent_image.rs`
- Modify: `crates/agent-kernel-core/src/agent_image_store.rs`

- [x] **Step 1: Add retirement red tests**

Append to `agent_image.rs`:

```rust
#[test]
fn retire_agent_image_marks_retired_and_records_event() {
    let mut core = ImageCore::new();
    let (owner, capability, resource) =
        prepare_owner(&mut core, OperationSet::empty().with(Operation::Act).with(Operation::Rollback));
    let image = core
        .register_agent_image(owner, capability, resource, AgentImageKind::Worker, digest(8), 1, 1)
        .expect("image should register");

    let event = core
        .retire_agent_image(owner, capability, image)
        .expect("image should retire");

    assert_eq!(event.kind, EventKind::AgentImageRetired);
    assert_eq!(event.agent_image, Some(image));
    assert_eq!(event.agent_image_kind, Some(AgentImageKind::Worker));
    assert_eq!(event.agent_image_digest, None);
    assert_eq!(event.agent_image_abi_version, None);
    assert_eq!(event.agent_image_entry_version, None);
    assert_eq!(
        core.agent_image(image).expect("image should remain queryable").status,
        AgentImageStatus::Retired
    );
}

#[test]
fn retire_agent_image_requires_owner_and_rollback_without_mutation() {
    let mut core = ImageCore::new();
    let (owner, capability, resource) =
        prepare_owner(&mut core, OperationSet::only(Operation::Act));
    let image = core
        .register_agent_image(owner, capability, resource, AgentImageKind::Worker, digest(9), 1, 1)
        .expect("image should register");
    let events_before = core.events().len();

    let result = core.retire_agent_image(owner, capability, image);

    assert_eq!(result, Err(KernelError::OperationDenied));
    assert_eq!(
        core.agent_image(image).expect("image should remain queryable").status,
        AgentImageStatus::Active
    );
    assert_eq!(core.events().len(), events_before);
}
```

- [x] **Step 2: Run retirement tests and verify red**

Run:

```bash
PATH="$HOME/.cargo/bin:$PATH" RUSTC="$(rustup which rustc --toolchain nightly)" rustup run nightly cargo test -p agent-kernel-core --test agent_image retire_agent_image
```

Expected: compile failure or method-not-found failure for `retire_agent_image`.

- [x] **Step 3: Implement retirement**

In `agent_image_store.rs`, add mutable lookup and retirement:

```rust
pub fn retire_agent_image(
    &mut self,
    owner: AgentId,
    capability: CapabilityId,
    image: AgentImageId,
) -> Result<Event, KernelError> {
    self.ensure_agent_active(owner)?;
    let record = self.find_agent_image(image)?;
    if record.status != AgentImageStatus::Active {
        return Err(KernelError::AgentImageRetired);
    }
    if record.owner != owner {
        return Err(KernelError::AgentMismatch);
    }
    self.ensure_authorized(owner, capability, record.resource, Operation::Rollback)?;
    self.ensure_event_slots(1)?;
    self.find_agent_image_mut(image)?.status = AgentImageStatus::Retired;
    self.record_agent_image_retired_event(owner, capability, record.resource, image, record.kind)
}
```

The retirement event sets `agent_image` and `agent_image_kind`; digest and version fields stay `None`.

- [x] **Step 4: Verify retirement green**

Run:

```bash
PATH="$HOME/.cargo/bin:$PATH" RUSTC="$(rustup which rustc --toolchain nightly)" rustup run nightly cargo test -p agent-kernel-core --test agent_image
```

Expected: all image registration and retirement tests pass.

## Task 4: Launch Binding And Image Validation

**Files:**
- Modify: `crates/agent-kernel-core/src/agent_entry.rs`
- Modify: `crates/agent-kernel-core/src/agent_launch.rs`
- Modify: `crates/agent-kernel-core/tests/agent_launch.rs`
- Modify: `crates/agent-kernel-core/tests/agent_launch_errors.rs`
- Modify: launch callers under `crates/agent-kernel-core/tests/`

- [x] **Step 1: Add launch image red tests**

In `crates/agent-kernel-core/tests/agent_launch.rs`, import `AgentImageDigest` and `AgentImageKind`. Add helper:

```rust
fn image_digest(byte: u8) -> AgentImageDigest {
    AgentImageDigest::new([byte; 32])
}
```

Update successful launch tests to register an image and call:

```rust
let image = core
    .register_agent_image(
        agent,
        capability,
        resource,
        AgentImageKind::Supervisor,
        image_digest(1),
        1,
        1,
    )
    .expect("image should register");
let event = core
    .launch_agent(agent, capability, resource, image, AgentEntryKind::Supervisor, None)
    .expect("agent should launch");
assert_eq!(event.agent_image, Some(image));
let entry = core.agent_entry(agent).expect("agent entry should exist");
assert_eq!(entry.image, image);
```

In `agent_launch_errors.rs`, add tests for:

```rust
assert_eq!(
    core.launch_agent(
        agent,
        capability,
        resource,
        AgentImageId::new(99),
        AgentEntryKind::Worker,
        None
    ),
    Err(KernelError::AgentImageNotFound)
);
```

and for resource mismatch and kind mismatch:

```rust
assert_eq!(result, Err(KernelError::AgentImageResourceMismatch));
assert_eq!(result, Err(KernelError::AgentImageKindMismatch));
```

- [x] **Step 2: Run launch tests and verify red**

Run:

```bash
PATH="$HOME/.cargo/bin:$PATH" RUSTC="$(rustup which rustc --toolchain nightly)" rustup run nightly cargo test -p agent-kernel-core --test agent_launch --test agent_launch_errors
```

Expected: compile failures because `launch_agent` does not accept `AgentImageId` and `AgentEntryRecord` has no `image`.

- [x] **Step 3: Implement launch image binding**

Update `AgentEntryRecord`:

```rust
pub image: AgentImageId,
```

Set `image: AgentImageId::new(0)` in `empty()`.

Change launch signatures:

```rust
pub fn launch_agent(
    &mut self,
    agent: AgentId,
    capability: CapabilityId,
    resource: ResourceId,
    image: AgentImageId,
    kind: AgentEntryKind,
    intent: Option<IntentId>,
) -> Result<Event, KernelError>
```

```rust
pub fn launch_task_agent(
    &mut self,
    agent: AgentId,
    capability: CapabilityId,
    task: TaskId,
    image: AgentImageId,
    kind: AgentEntryKind,
) -> Result<Event, KernelError>
```

Before capacity mutation, validate:

```rust
self.ensure_launch_image(image, resource, kind)?;
```

Record entries with `image`, and change `record_agent_launch_event` to accept and store `agent_image: Some(image)`.

In `agent_image_store.rs`, add:

```rust
pub(crate) fn ensure_launch_image(
    &self,
    image: AgentImageId,
    resource: ResourceId,
    entry_kind: AgentEntryKind,
) -> Result<AgentImageRecord, KernelError> {
    let record = self.find_agent_image(image)?;
    if record.status != AgentImageStatus::Active {
        return Err(KernelError::AgentImageRetired);
    }
    if record.resource != resource {
        return Err(KernelError::AgentImageResourceMismatch);
    }
    if !image_kind_matches_entry(record.kind, entry_kind) {
        return Err(KernelError::AgentImageKindMismatch);
    }
    Ok(record)
}
```

Use direct kind mapping in a small private helper.

- [x] **Step 4: Migrate core launch callers**

For each `launch_agent` or `launch_task_agent` call found by:

```bash
rg -n "launch_agent\\(|launch_task_agent\\(" crates/agent-kernel-core/tests -g '*.rs'
```

register a matching image immediately before launch using the same agent, resource, and capability. For task-scoped launches, use `AgentImageKind::Worker` and the task resource.

- [x] **Step 5: Verify core launch green**

Run:

```bash
PATH="$HOME/.cargo/bin:$PATH" RUSTC="$(rustup which rustc --toolchain nightly)" rustup run nightly cargo test -p agent-kernel-core
```

Expected: all core tests pass.

## Task 5: Facade Image Syscalls

**Files:**
- Modify: `crates/agent-kernel/src/lib.rs`
- Modify: `crates/agent-kernel/src/agent.rs`
- Modify: `crates/agent-kernel/tests/agent_launch.rs`
- Modify: `crates/agent-kernel/tests/kernel_facade.rs`
- Modify: launch callers under `crates/agent-kernel/tests/`

- [x] **Step 1: Write facade red tests**

In `crates/agent-kernel/tests/agent_launch.rs`, register an image through the facade:

```rust
let image = kernel
    .sys_register_agent_image(
        agent,
        capability,
        resource,
        AgentImageKind::Supervisor,
        AgentImageDigest::new([11; 32]),
        1,
        1,
    )
    .expect("image should register through facade");
let event = kernel
    .sys_launch_agent(agent, capability, resource, image, AgentEntryKind::Supervisor, None)
    .expect("agent should launch with image through facade");
assert_eq!(event.agent_image, Some(image));
assert_eq!(kernel.agent_image(image).expect("image should be queryable").digest, AgentImageDigest::new([11; 32]));
```

- [x] **Step 2: Run facade tests and verify red**

Run:

```bash
PATH="$HOME/.cargo/bin:$PATH" RUSTC="$(rustup which rustc --toolchain nightly)" rustup run nightly cargo test -p agent-kernel --test agent_launch
```

Expected: compile failure for missing facade image syscalls and launch signatures.

- [x] **Step 3: Implement facade wrappers and generic propagation**

Append `const AGENT_IMAGES: usize = 0` after `WAITERS` in `AgentKernel`, pass it through to `KernelCore`, and update all facade impl generic headers.

In `agent.rs`, import image types and add:

```rust
pub fn sys_register_agent_image(
    &mut self,
    owner: AgentId,
    capability: CapabilityId,
    resource: ResourceId,
    kind: AgentImageKind,
    digest: AgentImageDigest,
    abi_version: u16,
    entry_version: u16,
) -> Result<AgentImageId, KernelError> {
    self.core.register_agent_image(owner, capability, resource, kind, digest, abi_version, entry_version)
}

pub fn sys_retire_agent_image(
    &mut self,
    owner: AgentId,
    capability: CapabilityId,
    image: AgentImageId,
) -> Result<Event, KernelError> {
    self.core.retire_agent_image(owner, capability, image)
}

pub fn agent_images(&self) -> &[AgentImageRecord] {
    self.core.agent_images()
}

pub fn agent_image(&self, image: AgentImageId) -> Result<AgentImageRecord, KernelError> {
    self.core.agent_image(image)
}
```

Update `sys_launch_agent` and `sys_launch_task_agent` to accept `AgentImageId`.

- [x] **Step 4: Migrate facade launch callers**

For each facade launch call found by:

```bash
rg -n "sys_launch_agent\\(|sys_launch_task_agent\\(" crates/agent-kernel/tests crates/agent-supervisor/src crates/agent-kernel-boot/src -g '*.rs'
```

register a matching image and pass the image id into the launch call.

- [x] **Step 5: Verify facade green**

Run:

```bash
PATH="$HOME/.cargo/bin:$PATH" RUSTC="$(rustup which rustc --toolchain nightly)" rustup run nightly cargo test -p agent-kernel
```

Expected: all facade tests pass.

## Task 6: Supervisor, Boot, Serial Output, And README

**Files:**
- Modify: `crates/agent-supervisor/src/main.rs`
- Modify: `crates/agent-supervisor/src/format.rs`
- Modify: `crates/agent-supervisor/src/format_agent.rs`
- Modify: `crates/agent-supervisor/src/flow_resources.rs`
- Modify: `crates/agent-supervisor/tests/supervisor_flow.rs`
- Modify: `crates/agent-kernel-boot/src/lib.rs`
- Modify: `crates/agent-kernel-boot/tests/boot_flow.rs`
- Modify: `crates/agent-kernel-x86_64/src/main.rs`
- Modify: `README.md`

- [x] **Step 1: Add supervisor image registration output**

In supervisor `main.rs`, import `AgentImageDigest` and `AgentImageKind`. After the owner capability grant, register:

```rust
let supervisor_image = kernel
    .sys_register_agent_image(
        agent,
        owner_capability,
        workspace,
        AgentImageKind::Supervisor,
        AgentImageDigest::new([1; 32]),
        1,
        1,
    )
    .expect("supervisor image should register");
```

After delegated capability derivation and before worker launch, register:

```rust
let worker_image = kernel
    .sys_register_agent_image(
        target_agent,
        assignee_capability,
        workspace,
        AgentImageKind::Worker,
        AgentImageDigest::new([2; 32]),
        1,
        1,
    )
    .expect("worker image should register");
```

Pass `supervisor_image` and `worker_image` to launch calls.

- [x] **Step 2: Render image events and launch image ids**

Add `EventKind::AgentImageRegistered` and `EventKind::AgentImageRetired` to `format.rs`. Add a formatter that renders:

```text
event[N] agent_image_registered agent=A resource=R capability=C image=I kind=K
event[N] agent_image_retired agent=A resource=R capability=C image=I kind=K
```

Update `format_agent_launch_event` so resource-scoped launch renders:

```text
event[N] agent_launched agent=A resource=R capability=C image=I
```

and task-scoped launch renders:

```text
event[N] agent_launched agent=A resource=R capability=C image=I task=T
```

- [x] **Step 3: Update boot flow**

In `agent-kernel-boot/src/lib.rs`, add `bootstrap_image: AgentImageId` to `BootReport`. Register a bootstrap image with digest `[0; 32]`, ABI version `1`, entry version `1`, and kind `AgentImageKind::Bootstrap` before launching the bootstrap agent. Pass that image into `sys_launch_agent`.

Update boot tests to expect seven events:

```rust
assert_eq!(events.len(), 7);
assert_eq!(events[0].kind, EventKind::AgentRegistered);
assert_eq!(events[1].kind, EventKind::CapabilityGranted);
assert_eq!(events[2].kind, EventKind::AgentImageRegistered);
assert_eq!(events[3].kind, EventKind::AgentLaunched);
assert_eq!(events[4].kind, EventKind::Observation);
assert_eq!(events[5].kind, EventKind::ActionExecuted);
assert_eq!(events[6].kind, EventKind::VerificationRequested);
```

- [x] **Step 4: Update x86 serial labels**

In `agent-kernel-x86_64/src/main.rs`, add match arms:

```rust
EventKind::AgentImageRegistered => {
    serial_write_line("agent_image_registered");
}
EventKind::AgentImageRetired => {
    serial_write_line("agent_image_retired");
}
```

- [x] **Step 5: Update README traces**

Update README current scope and behavior text to include agent images. Update boot trace to include:

```text
event[3] agent_image_registered
event[4] agent_launched
```

Update supervisor trace around launch points to include:

```text
event[5] agent_image_registered agent=1 resource=1 capability=1 image=1 kind=supervisor
event[6] agent_launched agent=1 resource=1 capability=1 image=1
```

and:

```text
event[19] agent_image_registered agent=2 resource=1 capability=2 image=2 kind=worker
event[20] agent_launched agent=2 resource=1 capability=2 image=2 task=1
```

- [x] **Step 6: Verify supervisor and boot green**

Run:

```bash
PATH="$HOME/.cargo/bin:$PATH" RUSTC="$(rustup which rustc --toolchain nightly)" rustup run nightly cargo test -p agent-supervisor -p agent-kernel-boot
```

Expected: supervisor and boot tests pass with image registration events in the output.

## Task 7: Workspace Validation And Commit

**Files:**
- Modify checked plan status in `docs/superpowers/plans/2026-07-06-agent-image-v0.md`
- Commit all implementation files

- [x] **Step 1: Format**

Run:

```bash
PATH="$HOME/.cargo/bin:$PATH" RUSTC="$(rustup which rustc --toolchain nightly)" rustup run nightly cargo fmt --check
```

Expected: no formatting diff.

- [x] **Step 2: Run full workspace tests**

Run:

```bash
PATH="$HOME/.cargo/bin:$PATH" RUSTC="$(rustup which rustc --toolchain nightly)" RUSTDOC="$(rustup which rustdoc --toolchain nightly)" rustup run nightly cargo test --workspace
```

Expected: all workspace tests pass.

- [x] **Step 3: Run supervisor**

Run:

```bash
PATH="$HOME/.cargo/bin:$PATH" RUSTC="$(rustup which rustc --toolchain nightly)" rustup run nightly cargo run -p agent-supervisor
```

Expected: output includes `agent_image_registered` before both launch events and each `agent_launched` line includes `image=`.

- [x] **Step 4: Run QEMU boot**

Run:

```bash
PATH="$HOME/.cargo/bin:$PATH" RUSTC="$(rustup which rustc --toolchain nightly)" RUSTDOC="$(rustup which rustdoc --toolchain nightly)" rustup run nightly ./scripts/run-qemu.sh
```

Expected: serial output includes `AGENT_KERNEL_QEMU_BOOT_OK`, `event[3] agent_image_registered`, `event[4] agent_launched`, and `SUPERVISOR_HANDOFF_READY`.

- [x] **Step 5: no_std and file-size checks**

Run:

```bash
rg -n "extern crate std|use std::|alloc::|Vec<|String|Box<|println!|format!|thread|fs::|env::|net::|SystemTime|HashMap" crates/agent-kernel-core/src crates/agent-kernel/src crates/agent-kernel-boot/src
find crates/agent-kernel-core/src crates/agent-kernel/src crates/agent-supervisor/src crates/agent-kernel-core/tests crates/agent-kernel/tests crates/agent-supervisor/tests crates/agent-kernel-boot/src crates/agent-kernel-boot/tests crates/agent-kernel-x86_64/src -name '*.rs' -print0 | xargs -0 wc -l
```

Expected: no forbidden no_std symbols in no_std crates. New files remain under hard limits: core modules under 400 lines, facade modules under 320 lines, supervisor modules under 450 lines, tests under 500 lines.

- [x] **Step 6: Commit and push**

Run:

```bash
git status --short
git add README.md crates/agent-kernel-core crates/agent-kernel crates/agent-supervisor crates/agent-kernel-boot crates/agent-kernel-x86_64 docs/superpowers/plans/2026-07-06-agent-image-v0.md
git diff --cached --check
git commit -m "feat: add agent image executable identity"
git push origin main
```

Expected: push advances `origin/main`.

## Self-Review

Spec coverage:

- Core image IDs, records, digest, kind, status, fixed-capacity store, registration, and retirement are covered by Tasks 1-3.
- Launch-time image validation, entry/event binding, retired image rejection, resource mismatch, and kind mismatch are covered by Task 4.
- Facade syscalls and query methods are covered by Task 5.
- Supervisor output, boot flow, x86 serial output, README, and QEMU validation are covered by Task 6 and Task 7.
- Replayability is covered by registration event digest/version assertions in Task 1.

Completeness scan:

- The plan contains concrete file paths, command lines, expected outcomes, public type names, and API names for each implementation task.

Type consistency:

- Public image types are `AgentImageId`, `AgentImageDigest`, `AgentImageKind`, `AgentImageStatus`, and `AgentImageRecord`.
- Core APIs are `register_agent_image`, `retire_agent_image`, `agent_images`, `agent_image`, `launch_agent(..., image, ...)`, and `launch_task_agent(..., image, ...)`.
- Facade APIs are `sys_register_agent_image`, `sys_retire_agent_image`, `agent_images`, `agent_image`, `sys_launch_agent(..., image, ...)`, and `sys_launch_task_agent(..., image, ...)`.
