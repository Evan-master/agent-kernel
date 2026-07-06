# Resource Retirement V0 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add explicit active/retired lifecycle state for Agent Kernel resources.

**Architecture:** `agent-kernel-core` owns resource status, active-resource lookup, retirement authorization, and retirement event recording. `agent-kernel` exposes syscall-style wrappers. Supervisor and QEMU format the new event label without introducing deletion, ID reuse, or host teardown semantics.

**Tech Stack:** Rust nightly, no_std core and facade crates, fixed-capacity arrays, Cargo workspace tests, QEMU BIOS boot script.

---

## File Structure

- Modify `crates/agent-kernel-core/src/resource.rs` for `ResourceStatus`.
- Modify `crates/agent-kernel-core/src/resource_store.rs` for `resources()` and `retire_resource`.
- Modify `crates/agent-kernel-core/src/lookup.rs` so retired resources fail active lookup.
- Modify `crates/agent-kernel-core/src/event.rs` for `ResourceRetired`.
- Modify `crates/agent-kernel-core/src/error.rs` for `ResourceRetired`.
- Modify `crates/agent-kernel-core/src/lib.rs` to export `ResourceStatus`.
- Modify `crates/agent-kernel/src/lib.rs` and `resource.rs` to expose `sys_retire_resource` and `resources()`.
- Add `crates/agent-kernel-core/tests/resource_retirement.rs`.
- Add `crates/agent-kernel-core/tests/resource_retirement_errors.rs`.
- Add `crates/agent-kernel/tests/resource_retirement.rs`.
- Modify `crates/agent-supervisor/src/main.rs`, `format.rs`, supervisor tests, QEMU serial output, README, and this plan.

## Task 1: Red Tests

- [x] **Step 1: Add core success-path test**

Create `crates/agent-kernel-core/tests/resource_retirement.rs` with a test that retires a resource, asserts `ResourceStatus::Retired`, and checks `ResourceRetired`.

- [x] **Step 2: Add core failure-path tests**

Create `crates/agent-kernel-core/tests/resource_retirement_errors.rs` with tests for missing rollback authority, event-log-full atomicity, and retired resource rejection for grant, old-capability observe, and child registration.

- [x] **Step 3: Add facade retirement test**

Create `crates/agent-kernel/tests/resource_retirement.rs` with a syscall-level retire flow and resource inspection assertion.

- [x] **Step 4: Verify red**

Run:

```bash
RUSTC="$(rustup which rustc --toolchain nightly)" RUSTDOC="$(rustup which rustdoc --toolchain nightly)" rustup run nightly cargo test -p agent-kernel-core --test resource_retirement
RUSTC="$(rustup which rustc --toolchain nightly)" RUSTDOC="$(rustup which rustdoc --toolchain nightly)" rustup run nightly cargo test -p agent-kernel-core --test resource_retirement_errors
RUSTC="$(rustup which rustc --toolchain nightly)" RUSTDOC="$(rustup which rustdoc --toolchain nightly)" rustup run nightly cargo test -p agent-kernel --test resource_retirement
```

Expected: fail because `ResourceStatus`, `ResourceRetired`, `retire_resource`, `resources`, and facade syscalls do not exist.

## Task 2: Core Resource Lifecycle

- [x] **Step 1: Add resource status**

Add `ResourceStatus::{Active, Retired}` and store status on every `Resource`.

- [x] **Step 2: Add active lookup semantics**

Update `find_resource` to return `KernelError::ResourceRetired` for retired resources.

- [x] **Step 3: Implement retirement**

Implement `resources()` and `retire_resource(agent, capability, resource)`. Retirement requires rollback authority, one event slot, marks status retired, and records `ResourceRetired`.

- [x] **Step 4: Verify focused core tests**

Run:

```bash
RUSTC="$(rustup which rustc --toolchain nightly)" RUSTDOC="$(rustup which rustdoc --toolchain nightly)" rustup run nightly cargo test -p agent-kernel-core --test resource_retirement
RUSTC="$(rustup which rustc --toolchain nightly)" RUSTDOC="$(rustup which rustdoc --toolchain nightly)" rustup run nightly cargo test -p agent-kernel-core --test resource_retirement_errors
```

Expected: pass.

## Task 3: Facade, Runtime, And Docs

- [x] **Step 1: Add facade syscall**

Add `sys_retire_resource` and `resources()` to `agent-kernel`.

- [x] **Step 2: Update supervisor flow**

Register a temporary service resource near the end of the supervisor flow, grant rollback authority for it, then retire it. The output should include `resource_retired`.

- [x] **Step 3: Update QEMU event label**

Add the serial label for `resource_retired`.

- [x] **Step 4: Update docs**

Update README behavior summary and expected supervisor output.

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

Expected: all commands pass; the no_std scan returns no matches; supervisor output includes `resource_retired` after the temporary service resource capability grant.

## Self-Review

Spec coverage: the plan covers resource status, retirement authority, active-resource lookup, event recording, facade syscall, runtime formatting, QEMU labels, and documentation.

Placeholder scan: no TODO, TBD, or open-ended implementation placeholders remain.

Type consistency: `ResourceStatus`, `ResourceRetired`, `retire_resource`, `sys_retire_resource`, and `resources()` are used consistently.
