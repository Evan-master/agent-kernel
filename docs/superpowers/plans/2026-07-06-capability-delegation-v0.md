# Capability Delegation V0 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add general agent-to-agent capability delegation with least-authority attenuation.

**Architecture:** `agent-kernel-core` owns source capability validation, subset checks, derived capability allocation, and event recording. `agent-kernel` exposes a syscall-style wrapper. Supervisor and README demonstrate the generic derived capability without changing bootstrap grant semantics.

**Tech Stack:** Rust nightly, no_std core and facade crates, fixed-capacity arrays, Cargo workspace tests, QEMU BIOS boot script.

---

## File Structure

- Modify `crates/agent-kernel-core/src/operation.rs` to add `OperationSet::is_subset_of`.
- Create `crates/agent-kernel-core/src/capability_derivation.rs` for `derive_capability` and task capability derivation.
- Modify `crates/agent-kernel-core/src/capability_store.rs` to keep grant, revoke, and capability event recording.
- Modify `crates/agent-kernel/src/lib.rs` to expose `sys_derive_capability`.
- Add `crates/agent-kernel-core/tests/capability_delegation.rs`.
- Add `crates/agent-kernel-core/tests/capability_delegation_errors.rs`.
- Add `crates/agent-kernel/tests/capability_delegation.rs`.
- Modify `crates/agent-supervisor/src/main.rs` and `tests/supervisor_flow.rs` to demonstrate general delegation.
- Modify `README.md` and this plan.

## Task 1: Red Tests

- [x] **Step 1: Add core success-path test**

Create `crates/agent-kernel-core/tests/capability_delegation.rs` with:

```rust
use agent_kernel_core::{
    AgentId, CapabilityId, EventKind, KernelCore, Operation, OperationSet, ResourceKind,
};

#[test]
fn derive_capability_records_event_and_target_can_use_subset_authority() {
    let mut core = KernelCore::<2, 1, 2, 5, 0, 1, 0, 0, 0, 0>::new();
    let owner = AgentId::new(1);
    let target = AgentId::new(2);
    core.register_agent(owner).expect("owner should register");
    core.register_agent(target).expect("target should register");
    let resource = core
        .register_resource(ResourceKind::Workspace, None)
        .expect("resource should fit");
    let source = core
        .grant_capability(
            owner,
            resource,
            OperationSet::empty()
                .with(Operation::Observe)
                .with(Operation::Delegate),
        )
        .expect("source capability should fit");

    let derived = core
        .derive_capability(owner, source, target, OperationSet::only(Operation::Observe))
        .expect("owner should derive observe authority");

    let event = core.events()[3];
    assert_eq!(event.kind, EventKind::CapabilityDerived);
    assert_eq!(event.agent, owner);
    assert_eq!(event.target_agent, Some(target));
    assert_eq!(event.resource, Some(resource));
    assert_eq!(event.capability, Some(derived));
    assert_eq!(event.source_capability, Some(source));
    assert_eq!(event.operations, OperationSet::only(Operation::Observe));
    assert_eq!(event.task, None);

    core.observe(target, derived, resource)
        .expect("target should use derived observe authority");
    assert_eq!(core.events()[4].kind, EventKind::Observation);
}
```

- [x] **Step 2: Add core failure-path tests**

Create `crates/agent-kernel-core/tests/capability_delegation_errors.rs` with tests for missing delegate authority, operation expansion, task-scoped source rejection, event-log-full atomicity, and source revocation invalidation.

- [x] **Step 3: Add facade test**

Create `crates/agent-kernel/tests/capability_delegation.rs` with a syscall-level flow that calls `sys_derive_capability` and then observes through the derived capability as the target agent.

- [x] **Step 4: Verify red**

Run:

```bash
RUSTC="$(rustup which rustc --toolchain nightly)" RUSTDOC="$(rustup which rustdoc --toolchain nightly)" rustup run nightly cargo test -p agent-kernel-core --test capability_delegation
RUSTC="$(rustup which rustc --toolchain nightly)" RUSTDOC="$(rustup which rustdoc --toolchain nightly)" rustup run nightly cargo test -p agent-kernel-core --test capability_delegation_errors
RUSTC="$(rustup which rustc --toolchain nightly)" RUSTDOC="$(rustup which rustdoc --toolchain nightly)" rustup run nightly cargo test -p agent-kernel --test capability_delegation
```

Expected: fail because `derive_capability` and `sys_derive_capability` do not exist.

## Task 2: Core Delegation

- [x] **Step 1: Add operation subset helper**

Add this method to `OperationSet`:

```rust
pub const fn is_subset_of(self, other: Self) -> bool {
    self.0 & !other.0 == 0
}
```

- [x] **Step 2: Implement `derive_capability`**

Add a public method to `capability_store.rs`:

```rust
pub fn derive_capability(
    &mut self,
    actor: AgentId,
    source_capability: CapabilityId,
    target_agent: AgentId,
    operations: OperationSet,
) -> Result<CapabilityId, KernelError>
```

It must validate active actor and target, require generic `Operation::Delegate` authority on the source capability resource, reject operation expansion with `KernelError::OperationDenied`, allocate a normal root capability with `parent: Some(source_capability)`, and record `CapabilityDerived`.

- [x] **Step 3: Verify focused core tests**

Run:

```bash
RUSTC="$(rustup which rustc --toolchain nightly)" RUSTDOC="$(rustup which rustdoc --toolchain nightly)" rustup run nightly cargo test -p agent-kernel-core --test capability_delegation --test capability_delegation_errors
```

Expected: pass.

## Task 3: Facade, Runtime, And Docs

- [x] **Step 1: Add facade syscall**

Add to `crates/agent-kernel/src/lib.rs`:

```rust
pub fn sys_derive_capability(
    &mut self,
    actor: AgentId,
    source_capability: CapabilityId,
    target_agent: AgentId,
    operations: OperationSet,
) -> Result<CapabilityId, KernelError> {
    self.core
        .derive_capability(actor, source_capability, target_agent, operations)
}
```

- [x] **Step 2: Update supervisor flow**

Near the end of `agent-supervisor`, derive an observe capability from the owner capability to the target agent, then let the target observe the workspace. Increase event capacity from `52` to `56`.

- [x] **Step 3: Update docs**

Update README current behavior and expected supervisor output with the generic capability derivation and target observation events.

- [x] **Step 4: Final verification**

Run:

```bash
rustup run nightly cargo fmt --check
RUSTC="$(rustup which rustc --toolchain nightly)" RUSTDOC="$(rustup which rustdoc --toolchain nightly)" rustup run nightly cargo test --workspace
RUSTC="$(rustup which rustc --toolchain nightly)" RUSTDOC="$(rustup which rustdoc --toolchain nightly)" rustup run nightly cargo run -p agent-supervisor
scripts/run-qemu.sh
git diff --check
rg -n "extern crate std|use std::|alloc::|Vec<|String|Box<|println!|format!|thread|fs::|env::|net::|SystemTime|HashMap" crates/agent-kernel-core/src crates/agent-kernel/src crates/agent-kernel-boot/src
git status --short --branch
```

Expected: all commands pass; no_std scan returns no matches; supervisor output includes a generic `capability_derived` event followed by a target-agent observation.

## Self-Review

Spec coverage: this plan covers source authority, operation attenuation, task-scope rejection, event atomicity, revocation propagation, facade exposure, runtime demonstration, and documentation.

Placeholder scan: no TODO, TBD, or open-ended implementation placeholders remain.

Type consistency: `derive_capability`, `sys_derive_capability`, `OperationSet::is_subset_of`, `CapabilityDerived`, and `Operation::Delegate` are used consistently.
