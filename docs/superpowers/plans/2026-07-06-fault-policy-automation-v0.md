# Fault Policy Automation V0 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add deterministic fault policies so the kernel can apply a configured native action to a fault.

**Architecture:** `agent-kernel-core` owns policy IDs, fixed-capacity policy records, install/apply authority checks, atomic route/recover policy application, and replayable policy events. `agent-kernel` exposes syscall-style wrappers. Supervisor and QEMU format the new labels without turning policy into host callbacks.

**Tech Stack:** Rust nightly, no_std core and facade crates, fixed-capacity arrays, Cargo workspace tests, QEMU BIOS boot script.

---

## File Structure

- Modify `crates/agent-kernel-core/src/id.rs` for `FaultPolicyId`.
- Create `crates/agent-kernel-core/src/fault_policy.rs` for `FaultPolicyAction`, `FaultPolicyRecord`, and `FaultPolicyOutcome`.
- Create `crates/agent-kernel-core/src/fault_policy_event.rs` for policy event construction.
- Create `crates/agent-kernel-core/src/fault_policy_store.rs` for install and apply behavior.
- Modify `crates/agent-kernel-core/src/core.rs`, `event.rs`, `error.rs`, `lib.rs`, `fault_store.rs`, and `fault_handler_store.rs`.
- Modify every `KernelCore<...>` impl header in core source to include trailing `FAULT_POLICIES` capacity.
- Modify `crates/agent-kernel/src/lib.rs`, `fault.rs`, `mailbox.rs`, `memory.rs`, `namespace.rs`, and `scheduler.rs` for `AgentKernel<..., FAULT_POLICIES>`.
- Add `crates/agent-kernel-core/tests/fault_policy.rs`.
- Add `crates/agent-kernel-core/tests/fault_policy_errors.rs`.
- Add `crates/agent-kernel-core/tests/fault_policy_install_errors.rs`.
- Add `crates/agent-kernel/tests/fault_policy.rs`.
- Modify `crates/agent-supervisor/src/main.rs`, `format.rs`, `format_fault.rs`, supervisor tests, QEMU serial output, README, and this design note.

## Task 1: Red Tests

- [x] **Step 1: Add core success-path tests**

Create `crates/agent-kernel-core/tests/fault_policy.rs` with tests for installing a route policy, applying a route policy, and applying a recover policy.

- [x] **Step 2: Add core failure-path tests**

Create `crates/agent-kernel-core/tests/fault_policy_errors.rs` with tests for missing rollback authority, duplicate policy binding, missing policy apply, route message-store-full atomicity, route event-log-full atomicity, and recover event-log-full atomicity.

- [x] **Step 3: Add facade policy test**

Create `crates/agent-kernel/tests/fault_policy.rs` with one syscall-level test that installs a handler, installs a route policy, faults a task, applies policy, receives and acknowledges the routed fault message, recovers, requeues, redispatches, and completes the task.

- [x] **Step 4: Verify red**

Run:

```bash
RUSTC="$(rustup which rustc --toolchain nightly)" RUSTDOC="$(rustup which rustdoc --toolchain nightly)" rustup run nightly cargo test -p agent-kernel-core --test fault_policy
RUSTC="$(rustup which rustc --toolchain nightly)" RUSTDOC="$(rustup which rustdoc --toolchain nightly)" rustup run nightly cargo test -p agent-kernel-core --test fault_policy_errors
RUSTC="$(rustup which rustc --toolchain nightly)" RUSTDOC="$(rustup which rustdoc --toolchain nightly)" rustup run nightly cargo test -p agent-kernel --test fault_policy
```

Expected: fail because `FaultPolicyId`, policy records, policy events, policy errors, core methods, event fields, and facade syscalls do not exist.

## Task 2: Core Policy Store

- [x] **Step 1: Add policy model**

Add `FaultPolicyId`, `FaultPolicyAction`, `FaultPolicyRecord`, and `FaultPolicyOutcome`.

- [x] **Step 2: Extend core storage and events**

Add trailing `FAULT_POLICIES` capacity, `fault_policies`, `fault_policy_len`, and `next_fault_policy` to `KernelCore`. Add `FaultPolicyInstalled`, `FaultPolicyApplied`, `fault_policy`, and `fault_policy_action`.

- [x] **Step 3: Add policy errors**

Add `FaultPolicyStoreFull`, `FaultPolicyAlreadyExists`, and `FaultPolicyNotFound`.

- [x] **Step 4: Expose crate-private fault and handler helpers**

Make the existing fault event builder and handler lookup helpers crate-private so policy application can preserve event ordering without duplicating routing logic.

- [x] **Step 5: Implement install and apply**

Implement `install_fault_policy`, `apply_fault_policy`, and `fault_policies()`. Route application records `MessageSent`, `FaultRouted`, and `FaultPolicyApplied`. Recover application records `TaskFaultRecovered` and `FaultPolicyApplied`.

- [x] **Step 6: Verify focused core tests**

Run:

```bash
RUSTC="$(rustup which rustc --toolchain nightly)" RUSTDOC="$(rustup which rustdoc --toolchain nightly)" rustup run nightly cargo test -p agent-kernel-core --test fault_policy
RUSTC="$(rustup which rustc --toolchain nightly)" RUSTDOC="$(rustup which rustdoc --toolchain nightly)" rustup run nightly cargo test -p agent-kernel-core --test fault_policy_errors
```

Expected: pass.

## Task 3: Facade, Runtime, And Docs

- [x] **Step 1: Add facade syscalls**

Add `sys_install_fault_policy`, `sys_apply_fault_policy`, and `fault_policies()` to `agent-kernel`.

- [x] **Step 2: Update supervisor flow**

Install a route policy after installing the fault handler. After task fault, call `sys_apply_fault_policy` instead of `sys_route_fault_to_handler`. The output should include `fault_policy_installed` and `fault_policy_applied`.

- [x] **Step 3: Update QEMU event labels**

Add serial labels for `fault_policy_installed` and `fault_policy_applied`.

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
rg -n "std::|Vec<|String|Box<|format!|println!|thread|fs::|env::|net::|SystemTime|HashMap" crates/agent-kernel-core/src crates/agent-kernel/src crates/agent-kernel-boot/src
git status --short --branch
```

Expected: all commands pass; the no_std scan returns no matches; supervisor output includes `fault_policy_installed`, `task_faulted`, `fault_routed`, `fault_policy_applied`, and the handler message receive/ack sequence before recovery.

## Self-Review

Spec coverage: the plan covers policy IDs, fixed-capacity policy records, install authority, duplicate rejection, route/recover actions, still-faulted validation, policy event metadata, route/recover event ordering, facade syscalls, runtime formatting, QEMU labels, and documentation.

Placeholder scan: no TODO, TBD, or open-ended implementation placeholders remain.

Type consistency: `FaultPolicyId`, `FaultPolicyAction`, `FaultPolicyRecord`, `FaultPolicyOutcome`, `FaultPolicyStoreFull`, `FaultPolicyAlreadyExists`, `FaultPolicyNotFound`, `FaultPolicyInstalled`, `FaultPolicyApplied`, `fault_policy`, `fault_policy_action`, `install_fault_policy`, `apply_fault_policy`, `sys_install_fault_policy`, and `sys_apply_fault_policy` are used consistently.
