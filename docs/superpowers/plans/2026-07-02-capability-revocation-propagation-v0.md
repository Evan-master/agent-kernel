# Capability Revocation Propagation V0 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make derived delegated task capabilities become unusable when their source capability is revoked.

**Architecture:** `agent-kernel-core` extends `Capability` with an optional parent id and performs bounded parent-chain revocation checks in authorization. `delegate_task` passes the source capability id into task capability derivation, while `revoke_capability` keeps mutating only the directly revoked grant. No facade syscall or supervisor flow changes are required.

**Tech Stack:** Rust nightly, no_std core and facade crates, fixed-capacity arrays, Cargo workspace tests, QEMU BIOS boot script.

---

## File Structure

- Modify `crates/agent-kernel-core/src/capability.rs` to add `parent: Option<CapabilityId>`.
- Modify `crates/agent-kernel-core/src/capability_store.rs` to initialize root and derived capability parents.
- Modify `crates/agent-kernel-core/src/task_store.rs` to pass the source capability id into derivation.
- Modify `crates/agent-kernel-core/src/authorization.rs` to reject revoked parent chains.
- Create `crates/agent-kernel-core/tests/capability_revocation.rs` for propagation behavior.
- Modify `README.md` to document source revocation invalidating derived delegated capabilities.

## Task 1: Red Tests For Revocation Propagation

**Files:**
- Create: `crates/agent-kernel-core/tests/capability_revocation.rs`

- [ ] **Step 1: Add propagation tests**

Create `crates/agent-kernel-core/tests/capability_revocation.rs`:

```rust
use agent_kernel_core::{
    AgentId, CapabilityId, KernelCore, KernelError, Operation, OperationSet, ResourceId,
    ResourceKind, TaskId, TaskStatus,
};

type TestCore = KernelCore<4, 8, 32, 6, 4>;

#[derive(Copy, Clone)]
struct RunningDelegatedTask {
    task: TaskId,
    source_capability: CapabilityId,
    delegated_capability: CapabilityId,
}

fn source_capability(
    core: &mut TestCore,
    owner: AgentId,
    resource: ResourceId,
) -> CapabilityId {
    core.grant_capability(
        owner,
        resource,
        OperationSet::empty()
            .with(Operation::Act)
            .with(Operation::Delegate)
            .with(Operation::Verify)
            .with(Operation::Rollback),
    )
    .expect("source capability should fit")
}

fn running_delegated_task(
    core: &mut TestCore,
    owner: AgentId,
    assignee: AgentId,
) -> RunningDelegatedTask {
    let resource = core
        .register_resource(ResourceKind::Workspace, None)
        .expect("resource should fit");
    let source_capability = source_capability(core, owner, resource);
    let task = core
        .create_task(owner, source_capability, resource)
        .expect("task should be created");
    let event = core
        .delegate_task(owner, source_capability, task, assignee)
        .expect("task should be delegated");
    let delegated_capability = event
        .capability
        .expect("delegation should expose derived capability");

    core.accept_task(assignee, task)
        .expect("task should be accepted");
    core.enqueue_task(assignee, task)
        .expect("task should enqueue");
    core.dispatch_next(assignee).expect("task should dispatch");

    RunningDelegatedTask {
        task,
        source_capability,
        delegated_capability,
    }
}

#[test]
fn revoking_source_capability_invalidates_derived_task_capability() {
    let mut core = TestCore::new();
    let owner = AgentId::new(1);
    let assignee = AgentId::new(2);
    let delegated = running_delegated_task(&mut core, owner, assignee);

    core.revoke_capability(delegated.source_capability)
        .expect("source capability should revoke");
    let events_before = core.events().len();

    let result = core.complete_task(assignee, delegated.delegated_capability, delegated.task);

    assert_eq!(result, Err(KernelError::CapabilityRevoked));
    assert_eq!(core.tasks()[0].status, TaskStatus::Running);
    assert_eq!(core.events().len(), events_before);
}

#[test]
fn revoking_derived_capability_rejects_task_completion() {
    let mut core = TestCore::new();
    let owner = AgentId::new(3);
    let assignee = AgentId::new(4);
    let delegated = running_delegated_task(&mut core, owner, assignee);

    core.revoke_capability(delegated.delegated_capability)
        .expect("derived capability should revoke");
    let events_before = core.events().len();

    let result = core.complete_task(assignee, delegated.delegated_capability, delegated.task);

    assert_eq!(result, Err(KernelError::CapabilityRevoked));
    assert_eq!(core.tasks()[0].status, TaskStatus::Running);
    assert_eq!(core.events().len(), events_before);
}

#[test]
fn revoking_unrelated_capability_does_not_invalidate_derived_task_capability() {
    let mut core = TestCore::new();
    let owner = AgentId::new(5);
    let assignee = AgentId::new(6);
    let delegated = running_delegated_task(&mut core, owner, assignee);
    let unrelated_resource = core
        .register_resource(ResourceKind::Device, None)
        .expect("unrelated resource should fit");
    let unrelated = core
        .grant_capability(owner, unrelated_resource, OperationSet::only(Operation::Act))
        .expect("unrelated capability should fit");

    core.revoke_capability(unrelated)
        .expect("unrelated capability should revoke");

    core.complete_task(assignee, delegated.delegated_capability, delegated.task)
        .expect("unrelated revocation should not affect derived task capability");

    assert_eq!(core.tasks()[0].status, TaskStatus::Completed);
}

#[test]
fn revoking_one_source_invalidates_multiple_derived_capabilities() {
    let mut core = TestCore::new();
    let owner = AgentId::new(7);
    let assignee = AgentId::new(8);
    let resource = core
        .register_resource(ResourceKind::Workspace, None)
        .expect("resource should fit");
    let source_capability = source_capability(&mut core, owner, resource);
    let first = core
        .create_task(owner, source_capability, resource)
        .expect("first task should be created");
    let first_capability = core
        .delegate_task(owner, source_capability, first, assignee)
        .expect("first task should delegate")
        .capability
        .expect("first delegation should derive capability");
    let second = core
        .create_task(owner, source_capability, resource)
        .expect("second task should be created");
    let second_capability = core
        .delegate_task(owner, source_capability, second, assignee)
        .expect("second task should delegate")
        .capability
        .expect("second delegation should derive capability");

    core.accept_task(assignee, first)
        .expect("first task should accept");
    core.enqueue_task(assignee, first)
        .expect("first task should enqueue");
    core.dispatch_next(assignee)
        .expect("first task should dispatch");
    core.accept_task(assignee, second)
        .expect("second task should accept");
    core.enqueue_task(assignee, second)
        .expect("second task should enqueue");
    core.dispatch_next(assignee)
        .expect("second task should dispatch");

    core.revoke_capability(source_capability)
        .expect("source capability should revoke");
    let events_before = core.events().len();

    assert_eq!(
        core.complete_task(assignee, first_capability, first),
        Err(KernelError::CapabilityRevoked)
    );
    assert_eq!(
        core.complete_task(assignee, second_capability, second),
        Err(KernelError::CapabilityRevoked)
    );
    assert_eq!(core.tasks()[0].status, TaskStatus::Running);
    assert_eq!(core.tasks()[1].status, TaskStatus::Running);
    assert_eq!(core.events().len(), events_before);
}
```

- [ ] **Step 2: Run red core test**

Run:

```bash
PATH="$HOME/.cargo/bin:$PATH" RUSTC="$(rustup which rustc --toolchain nightly)" rustup run nightly cargo test -p agent-kernel-core --test capability_revocation
```

Expected: at least `revoking_source_capability_invalidates_derived_task_capability` fails because the derived task capability still completes after the source capability is revoked.

## Task 2: Parent Metadata And Authorization Check

**Files:**
- Modify: `crates/agent-kernel-core/src/capability.rs`
- Modify: `crates/agent-kernel-core/src/capability_store.rs`
- Modify: `crates/agent-kernel-core/src/task_store.rs`
- Modify: `crates/agent-kernel-core/src/authorization.rs`

- [ ] **Step 1: Extend `Capability`**

Change `crates/agent-kernel-core/src/capability.rs` to:

```rust
use crate::{AgentId, CapabilityId, OperationSet, ResourceId, TaskId};

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Capability {
    pub id: CapabilityId,
    pub agent: AgentId,
    pub resource: ResourceId,
    pub operations: OperationSet,
    pub revoked: bool,
    pub task: Option<TaskId>,
    pub parent: Option<CapabilityId>,
}
```

- [ ] **Step 2: Initialize root and derived parents**

In `crates/agent-kernel-core/src/capability_store.rs`, set root grants to
`parent: None`, change `derive_task_capability` to accept `parent:
CapabilityId`, and set `parent: Some(parent)`.

- [ ] **Step 3: Pass source capability during delegation**

In `crates/agent-kernel-core/src/task_store.rs`, pass the source `capability`
argument into `derive_task_capability`.

- [ ] **Step 4: Add bounded chain validation**

In `crates/agent-kernel-core/src/authorization.rs`, update
`ensure_capability_base` to call a helper like:

```rust
fn ensure_capability_chain_active(&self, capability: Capability) -> Result<(), KernelError> {
    let mut current = capability;

    for _ in 0..CAPS {
        if current.revoked {
            return Err(KernelError::CapabilityRevoked);
        }
        let Some(parent) = current.parent else {
            return Ok(());
        };
        current = self.find_capability(parent)?;
    }

    Err(KernelError::CapabilityNotFound)
}
```

The direct agent, resource, operation, and task-scope checks remain based on the
capability id supplied by the caller.

- [ ] **Step 5: Run focused core test**

Run:

```bash
PATH="$HOME/.cargo/bin:$PATH" RUSTC="$(rustup which rustc --toolchain nightly)" rustup run nightly cargo test -p agent-kernel-core --test capability_revocation
```

Expected: all tests in `capability_revocation` pass.

## Task 3: Documentation And Full Verification

**Files:**
- Modify: `README.md`

- [ ] **Step 1: Update README behavior**

Add one sentence after the existing delegation paragraph:

```markdown
Revoking the source capability that authorized delegation also invalidates the
derived task-scoped capability before future task authorization succeeds.
```

- [ ] **Step 2: Run formatting**

Run:

```bash
PATH="$HOME/.cargo/bin:$PATH" rustup run nightly cargo fmt --check
```

Expected: command exits successfully.

- [ ] **Step 3: Run workspace tests**

Run:

```bash
PATH="$HOME/.cargo/bin:$PATH" RUSTC="$(rustup which rustc --toolchain nightly)" rustup run nightly cargo test --workspace
```

Expected: command exits successfully.

- [ ] **Step 4: Run supervisor flow**

Run:

```bash
PATH="$HOME/.cargo/bin:$PATH" RUSTC="$(rustup which rustc --toolchain nightly)" rustup run nightly cargo run -p agent-supervisor
```

Expected: supervisor prints the same 12-event flow ending with `task_verified`.

- [ ] **Step 5: Run QEMU boot**

Run:

```bash
scripts/run-qemu.sh
```

Expected: serial output includes `AGENT_KERNEL_QEMU_BOOT_OK` and
`SUPERVISOR_HANDOFF_READY`.

- [ ] **Step 6: Commit and push**

Run:

```bash
git status --short
git add README.md \
  crates/agent-kernel-core/src/authorization.rs \
  crates/agent-kernel-core/src/capability.rs \
  crates/agent-kernel-core/src/capability_store.rs \
  crates/agent-kernel-core/src/task_store.rs \
  crates/agent-kernel-core/tests/capability_revocation.rs \
  docs/superpowers/specs/2026-07-02-capability-revocation-propagation-v0-design.md \
  docs/superpowers/plans/2026-07-02-capability-revocation-propagation-v0.md
git commit -m "feat: propagate capability revocation"
git push
```

Expected: commit is created and pushed to `origin/main`.

## Self-Review

Spec coverage:

- Parent metadata is covered by Task 2.
- Delegation parent assignment is covered by Task 2.
- Revoked parent-chain rejection is covered by Task 1 and Task 2.
- Direct derived revocation and unrelated revocation are covered by Task 1.
- README documentation impact is covered by Task 3.

Placeholder scan:

- The plan contains no placeholder markers or unexpanded test instruction.

Type consistency:

- `CapabilityId`, `TaskId`, `OperationSet`, and `KernelError` names match the
  existing `agent-kernel-core` API.
