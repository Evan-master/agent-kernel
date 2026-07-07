# Agent Image Verification V0 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make registered agent images non-launchable until an authorized kernel-visible verification transition records `AgentImageVerified`.

**Architecture:** Keep the behavior in `agent-kernel-core`: the image store owns lifecycle transitions, the image event helper owns replay event construction, and launch validation consumes verified image state. The `agent-kernel` facade exposes one syscall-style wrapper, while boot, supervisor, QEMU, tests, and README are callers that must verify images before launch.

**Tech Stack:** Rust workspace, `#![no_std]` core/facade crates, fixed-capacity stores, Cargo tests, QEMU script.

---

## File Structure

- Modify `crates/agent-kernel-core/src/agent_image.rs`: replace `Active` with `Pending` and `Verified`.
- Modify `crates/agent-kernel-core/src/event.rs`: add `EventKind::AgentImageVerified`.
- Modify `crates/agent-kernel-core/src/agent_image_event.rs`: add the verified image event recorder.
- Modify `crates/agent-kernel-core/src/agent_image_store.rs`: register pending images, verify pending images, launch only verified images, retire pending or verified images.
- Modify `crates/agent-kernel-core/src/agent_launch.rs`: no API change; it consumes the new launch image status contract through `ensure_launch_image`.
- Modify `crates/agent-kernel/src/agent.rs`: add `sys_verify_agent_image`.
- Modify focused tests:
  - `crates/agent-kernel-core/tests/agent_image.rs`
  - `crates/agent-kernel-core/tests/agent_launch.rs`
  - `crates/agent-kernel-core/tests/agent_launch_errors.rs`
  - `crates/agent-kernel/tests/kernel_facade.rs`
  - `crates/agent-kernel-boot/tests/boot_flow.rs`
- Modify all existing launch test setup files reported by `rg "register_agent_image\\(|sys_register_agent_image\\(" crates/*/tests crates/agent-supervisor/src crates/agent-kernel-boot/src`.
- Modify output and host flow files:
  - `crates/agent-supervisor/src/main.rs`
  - `crates/agent-supervisor/src/format.rs`
  - `crates/agent-kernel-boot/src/lib.rs`
  - `crates/agent-kernel-x86_64/src/main.rs`
  - `README.md`

## Task 1: Core Image Verification State Machine

**Files:**
- Modify: `crates/agent-kernel-core/tests/agent_image.rs`
- Modify: `crates/agent-kernel-core/src/agent_image.rs`
- Modify: `crates/agent-kernel-core/src/event.rs`
- Modify: `crates/agent-kernel-core/src/agent_image_event.rs`
- Modify: `crates/agent-kernel-core/src/agent_image_store.rs`

- [ ] **Step 1: Write the failing core image tests**

In `crates/agent-kernel-core/tests/agent_image.rs`, change the registration test status assertion:

```rust
assert_eq!(image_record.status, AgentImageStatus::Pending);
```

In the same file, append these tests:

```rust
#[test]
fn verify_agent_image_marks_verified_and_records_event() {
    let mut core = ImageCore::new();
    let (owner, capability, resource) = prepare_owner(
        &mut core,
        OperationSet::empty()
            .with(Operation::Act)
            .with(Operation::Verify),
    );
    let image = core
        .register_agent_image(
            owner,
            capability,
            resource,
            AgentImageKind::Worker,
            digest(10),
            1,
            1,
        )
        .expect("image should register");

    let event = core
        .verify_agent_image(owner, capability, image)
        .expect("image should verify");

    assert_eq!(event.kind, EventKind::AgentImageVerified);
    assert_eq!(event.agent, owner);
    assert_eq!(event.resource, Some(resource));
    assert_eq!(event.capability, Some(capability));
    assert_eq!(event.agent_image, Some(image));
    assert_eq!(event.agent_image_kind, Some(AgentImageKind::Worker));
    assert_eq!(event.agent_image_digest, None);
    assert_eq!(event.agent_image_abi_version, None);
    assert_eq!(event.agent_image_entry_version, None);
    assert_eq!(
        core.agent_image(image)
            .expect("image should remain queryable")
            .status,
        AgentImageStatus::Verified
    );
}

#[test]
fn verify_agent_image_requires_verify_authority_without_mutation() {
    let mut core = ImageCore::new();
    let (owner, capability, resource) =
        prepare_owner(&mut core, OperationSet::only(Operation::Act));
    let image = core
        .register_agent_image(
            owner,
            capability,
            resource,
            AgentImageKind::Worker,
            digest(11),
            1,
            1,
        )
        .expect("image should register");
    let events_before = core.events().len();

    let result = core.verify_agent_image(owner, capability, image);

    assert_eq!(result, Err(KernelError::OperationDenied));
    assert_eq!(
        core.agent_image(image)
            .expect("image should remain queryable")
            .status,
        AgentImageStatus::Pending
    );
    assert_eq!(core.events().len(), events_before);
}

#[test]
fn verify_agent_image_rejects_non_owner_without_mutation() {
    let mut core = ImageCore::new();
    let owner = AgentId::new(1);
    let other = AgentId::new(2);
    let (owner_capability, resource) = prepare_owner(
        &mut core,
        OperationSet::empty()
            .with(Operation::Act)
            .with(Operation::Verify),
    );
    core.register_agent(other)
        .expect("other agent should register");
    let other_capability = core
        .grant_capability(other, resource, OperationSet::only(Operation::Verify))
        .expect("other capability should fit");
    assert_eq!(owner, AgentId::new(1));
    let image = core
        .register_agent_image(
            owner,
            owner_capability,
            resource,
            AgentImageKind::Worker,
            digest(12),
            1,
            1,
        )
        .expect("image should register");
    let events_before = core.events().len();

    let result = core.verify_agent_image(other, other_capability, image);

    assert_eq!(result, Err(KernelError::AgentMismatch));
    assert_eq!(
        core.agent_image(image)
            .expect("image should remain queryable")
            .status,
        AgentImageStatus::Pending
    );
    assert_eq!(core.events().len(), events_before);
}

#[test]
fn verify_agent_image_rejects_repeated_or_retired_status_without_mutation() {
    let mut core = ImageCore::new();
    let (owner, capability, resource) = prepare_owner(
        &mut core,
        OperationSet::empty()
            .with(Operation::Act)
            .with(Operation::Verify)
            .with(Operation::Rollback),
    );
    let verified_image = core
        .register_agent_image(
            owner,
            capability,
            resource,
            AgentImageKind::Worker,
            digest(13),
            1,
            1,
        )
        .expect("verified image should register");
    core.verify_agent_image(owner, capability, verified_image)
        .expect("image should verify once");
    let events_after_verify = core.events().len();

    let repeated = core.verify_agent_image(owner, capability, verified_image);

    assert_eq!(repeated, Err(KernelError::AgentImageStatusMismatch));
    assert_eq!(core.events().len(), events_after_verify);
    assert_eq!(
        core.agent_image(verified_image)
            .expect("image should remain queryable")
            .status,
        AgentImageStatus::Verified
    );

    let retired_image = core
        .register_agent_image(
            owner,
            capability,
            resource,
            AgentImageKind::Worker,
            digest(14),
            1,
            1,
        )
        .expect("retired image should register");
    core.retire_agent_image(owner, capability, retired_image)
        .expect("pending image should retire");
    let events_after_retire = core.events().len();

    let retired = core.verify_agent_image(owner, capability, retired_image);

    assert_eq!(retired, Err(KernelError::AgentImageRetired));
    assert_eq!(core.events().len(), events_after_retire);
}

#[test]
fn verify_agent_image_event_log_full_leaves_pending() {
    let mut core = KernelCore::<2, 2, 4, 4, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 2>::new();
    let owner = AgentId::new(1);
    core.register_agent(owner).expect("owner should register");
    let resource = core
        .register_resource(ResourceKind::Workspace, None)
        .expect("resource should fit");
    let capability = core
        .grant_capability(
            owner,
            resource,
            OperationSet::empty()
                .with(Operation::Act)
                .with(Operation::Verify),
        )
        .expect("capability should fit");
    let image = core
        .register_agent_image(
            owner,
            capability,
            resource,
            AgentImageKind::Worker,
            digest(15),
            1,
            1,
        )
        .expect("image should register");

    let result = core.verify_agent_image(owner, capability, image);

    assert_eq!(result, Err(KernelError::EventLogFull));
    assert_eq!(
        core.agent_image(image)
            .expect("image should remain queryable")
            .status,
        AgentImageStatus::Pending
    );
    assert_eq!(core.events().len(), 4);
}
```

Update the existing retirement mutation assertion in `retire_agent_image_requires_rollback_without_mutation`:

```rust
assert_eq!(
    core.agent_image(image)
        .expect("image should remain queryable")
        .status,
    AgentImageStatus::Pending
);
```

- [ ] **Step 2: Run the focused failing test**

Run:

```bash
cargo test -p agent-kernel-core --test agent_image
```

Expected: FAIL to compile because `AgentImageStatus::Pending`, `AgentImageStatus::Verified`, `EventKind::AgentImageVerified`, and `verify_agent_image` do not exist.

- [ ] **Step 3: Implement the core status and event types**

In `crates/agent-kernel-core/src/agent_image.rs`, replace `AgentImageStatus` with:

```rust
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum AgentImageStatus {
    Pending,
    Verified,
    Retired,
}
```

In the `AgentImageRecord::empty()` return value, keep empty slots retired:

```rust
status: AgentImageStatus::Retired,
```

In `crates/agent-kernel-core/src/event.rs`, insert the new event kind between registration and retirement:

```rust
pub enum EventKind {
    AgentRegistered,
    AgentImageRegistered,
    AgentImageVerified,
    AgentImageRetired,
    AgentLaunched,
    AgentSuspended,
    AgentResumed,
    AgentRetired,
    ResourceCreated,
    ResourceRetired,
    CapabilityGranted,
    CapabilityDerived,
    CapabilityRevoked,
    IntentDeclared,
    IntentBound,
    IntentFulfilled,
    IntentCancelled,
    Observation,
    ActionExecuted,
    VerificationRequested,
    CheckpointCreated,
    RollbackRequested,
    DelegationRequested,
    TaskCreated,
    TaskAccepted,
    TaskCompleted,
    TaskVerified,
    TaskCancelled,
    TaskQueued,
    TaskDispatched,
    TaskYielded,
    TaskTicked,
    TaskQuantumExpired,
    TaskWaiting,
    TaskWoken,
    TaskFaulted,
    TaskFaultRecovered,
    SignalEmitted,
    FaultHandlerInstalled,
    FaultRouted,
    FaultPolicyInstalled,
    FaultPolicyApplied,
    MessageSent,
    MessageReceived,
    MessageAcknowledged,
    MemoryCellCreated,
    MemoryCellRecalled,
    MemoryCellRemembered,
    NamespaceEntryBound,
    NamespaceEntryResolved,
    NamespaceEntryRebound,
}
```

- [ ] **Step 4: Implement the verified image event recorder**

In `crates/agent-kernel-core/src/agent_image_event.rs`, add this method after `record_agent_image_registered_event`:

```rust
pub(crate) fn record_agent_image_verified_event(
    &mut self,
    owner: AgentId,
    capability: CapabilityId,
    resource: ResourceId,
    image: AgentImageId,
    kind: AgentImageKind,
) -> Result<Event, KernelError> {
    self.record(Event {
        sequence: 0,
        agent: owner,
        kind: EventKind::AgentImageVerified,
        resource: Some(resource),
        capability: Some(capability),
        source_capability: None,
        intent: None,
        intent_kind: None,
        action: None,
        observation: None,
        message: None,
        memory_cell: None,
        namespace_entry: None,
        namespace_key: None,
        namespace_object: None,
        operation: Some(Operation::Verify),
        operations: OperationSet::empty(),
        verification: VerificationRequirement::Optional,
        checkpoint: None,
        task: None,
        task_ticks: None,
        task_quantum: None,
        fault: None,
        fault_kind: None,
        fault_detail: None,
        fault_policy: None,
        fault_policy_action: None,
        waiter: None,
        signal: None,
        target_agent: None,
        agent_image: Some(image),
        agent_image_kind: Some(kind),
        agent_image_digest: None,
        agent_image_abi_version: None,
        agent_image_entry_version: None,
    })
}
```

- [ ] **Step 5: Implement registration, verification, and retirement transitions**

In `crates/agent-kernel-core/src/agent_image_store.rs`, set registered status to pending:

```rust
status: AgentImageStatus::Pending,
```

Add this method before `retire_agent_image`:

```rust
pub fn verify_agent_image(
    &mut self,
    owner: AgentId,
    capability: CapabilityId,
    image: AgentImageId,
) -> Result<Event, KernelError> {
    self.ensure_agent_active(owner)?;
    let record = self.find_agent_image(image)?;
    if record.owner != owner {
        return Err(KernelError::AgentMismatch);
    }
    match record.status {
        AgentImageStatus::Pending => {}
        AgentImageStatus::Verified => return Err(KernelError::AgentImageStatusMismatch),
        AgentImageStatus::Retired => return Err(KernelError::AgentImageRetired),
    }
    self.ensure_authorized(owner, capability, record.resource, Operation::Verify)?;
    self.ensure_event_slots(1)?;

    self.find_agent_image_mut(image)?.status = AgentImageStatus::Verified;
    self.record_agent_image_verified_event(
        owner,
        capability,
        record.resource,
        image,
        record.kind,
    )
}
```

In `retire_agent_image`, replace the active-only status check with:

```rust
match record.status {
    AgentImageStatus::Pending | AgentImageStatus::Verified => {}
    AgentImageStatus::Retired => return Err(KernelError::AgentImageRetired),
}
```

- [ ] **Step 6: Run the focused test to verify it passes**

Run:

```bash
cargo test -p agent-kernel-core --test agent_image
```

Expected: PASS for all tests in `agent_image.rs`.

- [ ] **Step 7: Commit the core state machine**

Run:

```bash
git add crates/agent-kernel-core/src/agent_image.rs \
  crates/agent-kernel-core/src/event.rs \
  crates/agent-kernel-core/src/agent_image_event.rs \
  crates/agent-kernel-core/src/agent_image_store.rs \
  crates/agent-kernel-core/tests/agent_image.rs
git commit -m "feat: verify agent image lifecycle"
```

## Task 2: Launch Gating For Verified Images

**Files:**
- Modify: `crates/agent-kernel-core/tests/agent_launch.rs`
- Modify: `crates/agent-kernel-core/tests/agent_launch_errors.rs`
- Modify: `crates/agent-kernel-core/src/agent_image_store.rs`

- [ ] **Step 1: Write launch gating tests and update success helpers**

In `crates/agent-kernel-core/tests/agent_launch.rs`, update `prepare_agent` to grant both act and verify:

```rust
let capability = core
    .grant_capability(
        agent,
        resource,
        OperationSet::empty()
            .with(Operation::Act)
            .with(Operation::Verify),
    )
    .expect("capability should fit");
```

After each successful `register_agent_image` in `agent_launch.rs`, verify before launching:

```rust
core.verify_agent_image(agent, capability, image)
    .expect("image should verify");
```

Update the first launch success event index because verification adds one event:

```rust
assert_eq!(core.events()[4], event);
```

In `crates/agent-kernel-core/tests/agent_launch_errors.rs`, change `register_with_capability` calls that should launch successfully to pass verify authority:

```rust
OperationSet::empty()
    .with(Operation::Act)
    .with(Operation::Verify)
```

Replace `register_image` with:

```rust
fn register_image(
    core: &mut TestCore,
    agent: AgentId,
    capability: CapabilityId,
    resource: ResourceId,
    kind: AgentImageKind,
) -> AgentImageId {
    let image = core
        .register_agent_image(agent, capability, resource, kind, digest(10), 1, 1)
        .expect("image should register");
    core.verify_agent_image(agent, capability, image)
        .expect("image should verify");
    image
}
```

Add this focused failure test:

```rust
#[test]
fn launch_rejects_pending_image_without_entry_or_event() {
    let mut core = TestCore::new();
    let agent = AgentId::new(1);
    let (capability, resource) = register_with_capability(
        &mut core,
        agent,
        OperationSet::empty()
            .with(Operation::Act)
            .with(Operation::Verify),
    );
    let image = core
        .register_agent_image(
            agent,
            capability,
            resource,
            AgentImageKind::Worker,
            digest(12),
            1,
            1,
        )
        .expect("image should register");
    let events_before = core.events().len();

    let result = core.launch_agent(
        agent,
        capability,
        resource,
        image,
        AgentEntryKind::Worker,
        None,
    );

    assert_eq!(result, Err(KernelError::AgentImageStatusMismatch));
    assert!(core.agent_entries().is_empty());
    assert_eq!(core.events().len(), events_before);
}
```

- [ ] **Step 2: Run focused launch tests and verify failure**

Run:

```bash
cargo test -p agent-kernel-core --test agent_launch --test agent_launch_errors
```

Expected: FAIL because `ensure_launch_image` still treats every non-active image as retired or still refers to `AgentImageStatus::Active`.

- [ ] **Step 3: Implement verified-only launch validation**

In `crates/agent-kernel-core/src/agent_image_store.rs`, replace the status check in `ensure_launch_image` with:

```rust
match record.status {
    AgentImageStatus::Verified => {}
    AgentImageStatus::Pending => return Err(KernelError::AgentImageStatusMismatch),
    AgentImageStatus::Retired => return Err(KernelError::AgentImageRetired),
}
```

- [ ] **Step 4: Run focused launch tests and verify pass**

Run:

```bash
cargo test -p agent-kernel-core --test agent_launch --test agent_launch_errors
```

Expected: PASS for both focused launch test files.

- [ ] **Step 5: Commit launch gating**

Run:

```bash
git add crates/agent-kernel-core/src/agent_image_store.rs \
  crates/agent-kernel-core/tests/agent_launch.rs \
  crates/agent-kernel-core/tests/agent_launch_errors.rs
git commit -m "feat: require verified images for launch"
```

## Task 3: Facade Verification Syscall

**Files:**
- Modify: `crates/agent-kernel/src/agent.rs`
- Modify: `crates/agent-kernel/tests/kernel_facade.rs`
- Modify: `crates/agent-kernel/tests/agent_launch.rs`

- [ ] **Step 1: Write facade tests**

In `crates/agent-kernel/tests/kernel_facade.rs`, add `AgentImageStatus` to the import list:

```rust
AgentImageId, AgentImageKind, AgentImageStatus,
```

Append this test:

```rust
#[test]
fn image_verification_syscall_exposes_verified_status_and_event() {
    let mut kernel = TestKernel::new();
    let agent = AgentId::new(120);
    kernel
        .sys_register_agent(agent)
        .expect("agent should register");
    let resource = kernel
        .sys_register_resource(ResourceKind::Workspace, None)
        .expect("resource should fit");
    let capability = kernel
        .sys_grant(
            agent,
            resource,
            OperationSet::empty()
                .with(Operation::Act)
                .with(Operation::Verify),
        )
        .expect("capability should fit");
    let image = kernel
        .sys_register_agent_image(
            agent,
            capability,
            resource,
            AgentImageKind::Worker,
            AgentImageDigest::new([9; 32]),
            1,
            1,
        )
        .expect("image should register");
    assert_eq!(
        kernel
            .agent_image(image)
            .expect("image should be queryable")
            .status,
        AgentImageStatus::Pending
    );

    let event = kernel
        .sys_verify_agent_image(agent, capability, image)
        .expect("image should verify");

    assert_eq!(event.kind, EventKind::AgentImageVerified);
    assert_eq!(event.agent_image, Some(image));
    assert_eq!(
        kernel
            .agent_image(image)
            .expect("image should be queryable")
            .status,
        AgentImageStatus::Verified
    );
}
```

In `crates/agent-kernel/tests/agent_launch.rs`, update the setup to call `sys_verify_agent_image` before `sys_launch_agent`:

```rust
kernel
    .sys_verify_agent_image(agent, capability, image)
    .expect("image should verify");
```

- [ ] **Step 2: Run facade tests and verify failure**

Run:

```bash
cargo test -p agent-kernel --test kernel_facade --test agent_launch
```

Expected: FAIL to compile because `sys_verify_agent_image` is not exposed by `AgentKernel`.

- [ ] **Step 3: Implement the syscall wrapper**

In `crates/agent-kernel/src/agent.rs`, add this method after `sys_register_agent_image`:

```rust
pub fn sys_verify_agent_image(
    &mut self,
    owner: AgentId,
    capability: CapabilityId,
    image: AgentImageId,
) -> Result<Event, KernelError> {
    self.core.verify_agent_image(owner, capability, image)
}
```

- [ ] **Step 4: Run facade tests and verify pass**

Run:

```bash
cargo test -p agent-kernel --test kernel_facade --test agent_launch
```

Expected: PASS for both facade test files.

- [ ] **Step 5: Commit facade syscall**

Run:

```bash
git add crates/agent-kernel/src/agent.rs \
  crates/agent-kernel/tests/kernel_facade.rs \
  crates/agent-kernel/tests/agent_launch.rs
git commit -m "feat: expose agent image verification syscall"
```

## Task 4: Boot, Supervisor, QEMU, And README Traces

**Files:**
- Modify: `crates/agent-kernel-boot/src/lib.rs`
- Modify: `crates/agent-kernel-boot/tests/boot_flow.rs`
- Modify: `crates/agent-supervisor/src/main.rs`
- Modify: `crates/agent-supervisor/src/format.rs`
- Modify: `crates/agent-kernel-x86_64/src/main.rs`
- Modify: `README.md`

- [ ] **Step 1: Write boot trace expectations**

In `crates/agent-kernel-boot/tests/boot_flow.rs`, update the event count and order:

```rust
assert_eq!(events.len(), 8);
assert_eq!(events[0].kind, EventKind::AgentRegistered);
assert_eq!(events[1].kind, EventKind::CapabilityGranted);
assert_eq!(events[2].kind, EventKind::AgentImageRegistered);
assert_eq!(events[3].kind, EventKind::AgentImageVerified);
assert_eq!(events[4].kind, EventKind::AgentLaunched);
assert_eq!(events[5].kind, EventKind::Observation);
assert_eq!(events[6].kind, EventKind::ActionExecuted);
assert_eq!(events[7].kind, EventKind::VerificationRequested);
assert_eq!(events[6].action, Some(ActionId::new(99)));
assert_eq!(events[7].action, Some(ActionId::new(99)));
```

- [ ] **Step 2: Run boot tests and verify failure**

Run:

```bash
cargo test -p agent-kernel-boot
```

Expected: FAIL because the boot flow registers and launches the bootstrap image without verification.

- [ ] **Step 3: Verify bootstrap image before launch**

In `crates/agent-kernel-boot/src/lib.rs`, insert after `sys_register_agent_image` and before `sys_launch_agent`:

```rust
kernel.sys_verify_agent_image(config.bootstrap_agent, capability, image)?;
```

- [ ] **Step 4: Update supervisor flow and formatting**

In `crates/agent-supervisor/src/main.rs`, insert after the supervisor image registration:

```rust
kernel
    .sys_verify_agent_image(agent, owner_capability, supervisor_image)
    .expect("supervisor image should verify");
```

Insert after the worker image registration:

```rust
kernel
    .sys_verify_agent_image(agent, owner_capability, worker_image)
    .expect("worker image should verify");
```

In `crates/agent-supervisor/src/format.rs`, add the new event arm after registration:

```rust
EventKind::AgentImageVerified => format_agent_image_event(event, "agent_image_verified"),
```

- [ ] **Step 5: Update QEMU serial event labels**

In `crates/agent-kernel-x86_64/src/main.rs`, add this match arm after `AgentImageRegistered`:

```rust
EventKind::AgentImageVerified => {
    serial_write_line("agent_image_verified");
}
```

- [ ] **Step 6: Update README traces**

In `README.md`, update every deterministic trace that lists `agent_image_registered` immediately before `agent_launched` to include:

```text
agent_image_verified
```

For prose that describes image registration as making the image launchable, replace it with:

```text
Agent images are registered as pending executable identities. A verifier-capable agent must verify the image before launch records can reference it.
```

- [ ] **Step 7: Run boot and supervisor validation**

Run:

```bash
cargo test -p agent-kernel-boot
cargo run -p agent-supervisor
```

Expected: both commands pass; supervisor output includes `agent_image_registered`, `agent_image_verified`, and `agent_launched` in that order for supervisor and worker images.

- [ ] **Step 8: Commit runtime trace updates**

Run:

```bash
git add crates/agent-kernel-boot/src/lib.rs \
  crates/agent-kernel-boot/tests/boot_flow.rs \
  crates/agent-supervisor/src/main.rs \
  crates/agent-supervisor/src/format.rs \
  crates/agent-kernel-x86_64/src/main.rs \
  README.md
git commit -m "feat: verify boot and supervisor images"
```

## Task 5: Migrate Existing Launch Test Setup

**Files:**
- Modify every test file that registers an image and then launches it:
  - `crates/agent-kernel-core/tests/agent_execution_context_errors.rs`
  - `crates/agent-kernel-core/tests/task_lifecycle.rs`
  - `crates/agent-kernel-core/tests/delegated_capability.rs`
  - `crates/agent-kernel-core/tests/agent_registry.rs`
  - `crates/agent-kernel-core/tests/scheduler_quantum.rs`
  - `crates/agent-kernel-core/tests/agent_lifecycle.rs`
  - `crates/agent-kernel-core/tests/task_fault_errors.rs`
  - `crates/agent-kernel-core/tests/scheduler_quantum_errors.rs`
  - `crates/agent-kernel-core/tests/intent_task_flow.rs`
  - `crates/agent-kernel-core/tests/fault_policy_errors.rs`
  - `crates/agent-kernel-core/tests/runtime_admission_errors.rs`
  - `crates/agent-kernel-core/tests/runtime_admission.rs`
  - `crates/agent-kernel-core/tests/task_wait_signal_errors.rs`
  - `crates/agent-kernel-core/tests/task_fault.rs`
  - `crates/agent-kernel-core/tests/fault_policy.rs`
  - `crates/agent-kernel-core/tests/task_authority.rs`
  - `crates/agent-kernel-core/tests/task_wait_signal_queue_errors.rs`
  - `crates/agent-kernel-core/tests/agent_execution_context.rs`
  - `crates/agent-kernel-core/tests/task_wait_signal.rs`
  - `crates/agent-kernel-core/tests/capability_revocation.rs`
  - `crates/agent-kernel-core/tests/fault_handler.rs`
  - `crates/agent-kernel-core/tests/fault_handler_errors.rs`
  - `crates/agent-kernel-core/tests/scheduler.rs`
  - `crates/agent-kernel/tests/task_wait_signal.rs`
  - `crates/agent-kernel/tests/task_fault.rs`
  - `crates/agent-kernel/tests/kernel_facade.rs`
  - `crates/agent-kernel/tests/agent_execution_context.rs`
  - `crates/agent-kernel/tests/fault_policy.rs`
  - `crates/agent-kernel/tests/runtime_admission.rs`
  - `crates/agent-kernel/tests/intent_lifecycle.rs`
  - `crates/agent-kernel/tests/fault_handler.rs`
  - `crates/agent-kernel/tests/scheduler_quantum.rs`

- [ ] **Step 1: Run the workspace tests to reveal migration failures**

Run:

```bash
cargo test --workspace
```

Expected: FAIL in tests that launch a pending image or use event indices that shifted by one.

- [ ] **Step 2: Apply the core-test migration pattern**

For every `crates/agent-kernel-core/tests/*.rs` file where a successful launch should still succeed, ensure the owner capability used for image registration includes verify authority:

```rust
OperationSet::empty()
    .with(Operation::Act)
    .with(Operation::Verify)
```

After each successful `register_agent_image(...)` whose image is passed to `launch_agent(...)` or `launch_task_agent(...)`, insert:

```rust
core.verify_agent_image(owner, owner_capability, image)
    .expect("image should verify");
```

When the variable names differ, keep the same ownership relation:

```rust
core.verify_agent_image(agent, capability, image)
    .expect("image should verify");
```

For delegated task launch tests where the owner registers the worker image and the assignee launches it, verify with the owner and owner capability before launch:

```rust
core.verify_agent_image(owner, owner_capability, worker_image)
    .expect("worker image should verify");
```

- [ ] **Step 3: Apply the facade-test migration pattern**

For every `crates/agent-kernel/tests/*.rs` file where a successful launch should still succeed, insert the facade verification call after image registration:

```rust
kernel
    .sys_verify_agent_image(owner, owner_capability, image)
    .expect("image should verify");
```

When the variables are named `agent`, `capability`, and `worker_image`, use:

```rust
kernel
    .sys_verify_agent_image(agent, capability, worker_image)
    .expect("worker image should verify");
```

- [ ] **Step 4: Update shifted event indexes**

Where tests assert exact event indexes after image registration and launch, add one index for each inserted `AgentImageVerified` event. Use explicit event-kind assertions rather than only length assertions. The expected local sequence around an image launch is:

```rust
assert_eq!(events[image_registered_index].kind, EventKind::AgentImageRegistered);
assert_eq!(events[image_registered_index + 1].kind, EventKind::AgentImageVerified);
assert_eq!(events[image_registered_index + 2].kind, EventKind::AgentLaunched);
```

- [ ] **Step 5: Run workspace tests and verify pass**

Run:

```bash
cargo test --workspace
```

Expected: PASS.

- [ ] **Step 6: Commit test migration**

Run:

```bash
git add crates/agent-kernel-core/tests crates/agent-kernel/tests
git commit -m "test: verify images before launch"
```

## Task 6: Final Validation And Publish

**Files:**
- Inspect: all modified files
- No production file changes unless validation exposes a defect

- [ ] **Step 1: Format check**

Run:

```bash
cargo fmt --check
```

Expected: PASS. If it fails, run `cargo fmt`, inspect the formatting diff, and commit formatting with the feature diff that introduced it.

- [ ] **Step 2: Full workspace tests**

Run:

```bash
cargo test --workspace
```

Expected: PASS.

- [ ] **Step 3: Supervisor run**

Run:

```bash
cargo run -p agent-supervisor
```

Expected: PASS and stdout contains at least one ordered subsequence:

```text
agent_image_registered
agent_image_verified
agent_launched
```

- [ ] **Step 4: QEMU boot**

Run:

```bash
./scripts/run-qemu.sh
```

Expected: PASS and serial output contains:

```text
AGENT_KERNEL_QEMU_BOOT_OK
event[2] agent_image_registered
event[3] agent_image_verified
event[4] agent_launched
```

- [ ] **Step 5: no_std forbidden symbol scan**

Run:

```bash
for crate in crates/agent-kernel-core crates/agent-kernel; do
  rg -n "Vec<|String|Box<|HashMap|Rc<|Arc<|std::|println!|format!|thread::|fs::|Tcp|Udp|async|await" "$crate/src" || true
done
```

Expected: no output.

- [ ] **Step 6: File-size check**

Run:

```bash
wc -l crates/agent-kernel-core/src/agent_image_store.rs \
  crates/agent-kernel-core/src/agent_image_event.rs \
  crates/agent-kernel-core/src/agent_image.rs \
  crates/agent-kernel/src/agent.rs
```

Expected: each source file stays below the handbook hard limit for its layer.

- [ ] **Step 7: Inspect git status and recent commits**

Run:

```bash
git status --short --branch
git log --oneline -5
```

Expected: worktree clean except intentional uncommitted fixes from validation. The recent commits should include:

```text
feat: verify agent image lifecycle
feat: require verified images for launch
feat: expose agent image verification syscall
feat: verify boot and supervisor images
test: verify images before launch
```

- [ ] **Step 8: Push**

Run:

```bash
git push origin main
```

Expected: push succeeds to `https://github.com/Evan-master/agent-kernel.git`.

## Self-Review

- Spec coverage: this plan covers pending registration, authorized verification, verified-only launch, retired behavior, facade syscall, boot/supervisor/QEMU trace updates, README updates, compatibility migration, and final validation.
- Placeholder scan: this plan contains no incomplete sections, no unspecified edge-case instructions, and no omitted test command.
- Type consistency: the plan uses `AgentImageStatus::{Pending, Verified, Retired}`, `EventKind::AgentImageVerified`, `verify_agent_image`, and `sys_verify_agent_image` consistently with the approved spec.
