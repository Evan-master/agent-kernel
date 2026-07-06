# Resource Ownership V0 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add owner-aware resource creation that atomically creates a resource and its first capability.

**Architecture:** `agent-kernel-core` owns resource owner metadata, owner-aware resource allocation, parent authorization, event ordering, and initial capability allocation. `agent-kernel` exposes the syscall wrapper. Supervisor uses owner-aware creation for the temporary service resource while bootstrap registration remains available for system-seeded resources.

**Tech Stack:** Rust nightly, no_std core and facade crates, fixed-capacity arrays, Cargo workspace tests, QEMU BIOS boot script.

---

## File Structure

- Modify `crates/agent-kernel-core/src/resource.rs` for `owner` and `ResourceCreateOutcome`.
- Create `crates/agent-kernel-core/src/resource_ownership.rs` for owner-aware creation.
- Modify `crates/agent-kernel-core/src/event.rs` for `ResourceCreated`.
- Modify `crates/agent-kernel-core/src/lib.rs` to export `ResourceCreateOutcome` and load `resource_ownership`.
- Modify `crates/agent-kernel/src/resource.rs` to expose `sys_create_resource`.
- Add `crates/agent-kernel-core/tests/resource_ownership.rs`.
- Add `crates/agent-kernel-core/tests/resource_ownership_errors.rs`.
- Add `crates/agent-kernel/tests/resource_ownership.rs`.
- Modify supervisor formatting, QEMU event labels, supervisor flow/tests, README, and this plan.

## Task 1: Red Tests

- [x] **Step 1: Add core success tests**

Create `crates/agent-kernel-core/tests/resource_ownership.rs` with tests proving root creation sets owner, records `ResourceCreated` before `CapabilityGranted`, and child creation stores parent and owner when the parent capability has `Act`.

- [x] **Step 2: Add core failure tests**

Create `crates/agent-kernel-core/tests/resource_ownership_errors.rs` with tests for inactive agent, missing parent act authority, event-log-full atomicity, capability-store-full atomicity, resource-store-full atomicity, and retired-parent rejection.

- [x] **Step 3: Add facade test**

Create `crates/agent-kernel/tests/resource_ownership.rs` with a syscall-level owner-aware creation flow and an observation through the returned capability.

- [x] **Step 4: Verify red**

Run:

```bash
RUSTC="$(rustup which rustc --toolchain nightly)" RUSTDOC="$(rustup which rustdoc --toolchain nightly)" rustup run nightly cargo test -p agent-kernel-core --test resource_ownership
RUSTC="$(rustup which rustc --toolchain nightly)" RUSTDOC="$(rustup which rustdoc --toolchain nightly)" rustup run nightly cargo test -p agent-kernel-core --test resource_ownership_errors
RUSTC="$(rustup which rustc --toolchain nightly)" RUSTDOC="$(rustup which rustdoc --toolchain nightly)" rustup run nightly cargo test -p agent-kernel --test resource_ownership
```

Expected: fail because `ResourceCreateOutcome`, `Resource.owner`, `ResourceCreated`, `create_resource`, and `sys_create_resource` do not exist.

## Task 2: Core Ownership

- [x] **Step 1: Add owner model**

Add `owner: Option<AgentId>` to `Resource`, add `ResourceCreateOutcome`, export it, and keep `register_resource` creating `owner: None`.

- [x] **Step 2: Implement owner-aware creation**

Create `resource_ownership.rs` with:

```rust
pub fn create_resource(
    &mut self,
    agent: AgentId,
    kind: ResourceKind,
    parent: Option<(ResourceId, CapabilityId)>,
    operations: OperationSet,
) -> Result<ResourceCreateOutcome, KernelError>
```

The method must validate active agent, parent `Act` authority when provided, resource/capability/event capacity, then insert the resource with `owner: Some(agent)`, insert the initial capability, record `ResourceCreated`, and record `CapabilityGranted`.

- [x] **Step 3: Verify focused core tests**

Run:

```bash
RUSTC="$(rustup which rustc --toolchain nightly)" RUSTDOC="$(rustup which rustdoc --toolchain nightly)" rustup run nightly cargo test -p agent-kernel-core --test resource_ownership --test resource_ownership_errors
```

Expected: pass.

## Task 3: Facade, Runtime, And Docs

- [x] **Step 1: Add facade syscall**

Add `sys_create_resource` to `crates/agent-kernel/src/resource.rs`.

- [x] **Step 2: Update runtime formatting**

Format `ResourceCreated` as:

```text
event[N] resource_created agent=1 resource=3 capability=4
```

Add the QEMU serial label `resource_created`.

- [x] **Step 3: Update supervisor flow**

Replace the temporary service resource `sys_register_resource` + `sys_grant` pair with `sys_create_resource(agent, ResourceKind::Service, Some((workspace, owner_capability)), OperationSet::only(Operation::Rollback))`. Expected new tail events:

```text
event[49] resource_created agent=1 resource=3 capability=4
event[50] capability_granted agent=1 resource=3 capability=4
event[51] resource_retired agent=1 resource=3 capability=4
event[52] capability_derived agent=1 resource=1 capability=5
event[53] observation agent=2 resource=1
```

- [x] **Step 4: Update docs**

Update README current behavior and expected supervisor output with owner-aware resource creation.

- [x] **Step 5: Final verification**

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

Expected: all commands pass; no_std scan returns no matches; supervisor output includes `resource_created` before the temporary service capability grant.

## Self-Review

Spec coverage: this plan covers owner metadata, root and child creation, parent authorization, atomicity, event ordering, facade exposure, runtime formatting, QEMU label, supervisor demo, and documentation.

Placeholder scan: no TODO, TBD, or open-ended implementation placeholders remain.

Type consistency: `ResourceCreateOutcome`, `owner`, `create_resource`, `sys_create_resource`, `ResourceCreated`, and `Operation::Act` are used consistently.
