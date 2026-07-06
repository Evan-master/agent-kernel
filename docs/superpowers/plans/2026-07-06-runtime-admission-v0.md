# Runtime Admission V0 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make launch entries the runtime admission boundary for task execution paths.

**Architecture:** `agent-kernel-core` owns admission checks and task-scoped launch records. Resource-scoped launch continues to cover bootstrap and supervisor agents, while task-scoped launch lets delegated workers run with only delegated task authority. `agent-kernel` exposes the syscall wrapper, and supervisor output demonstrates both owner and worker admission events.

**Tech Stack:** Rust nightly, no_std core and facade crates, fixed-capacity arrays, Cargo workspace tests, QEMU BIOS boot script.

---

## File Structure

- Modify `crates/agent-kernel-core/src/agent_entry.rs` to add `task: Option<TaskId>`.
- Modify `crates/agent-kernel-core/src/agent_launch.rs` to add task-scoped launch.
- Add `crates/agent-kernel-core/src/agent_admission.rs` for runtime admission helpers.
- Modify `crates/agent-kernel-core/src/error.rs` to add admission errors.
- Modify scheduler, tick, signal, fault, and task completion modules to call admission checks before runtime mutation.
- Modify `crates/agent-kernel/src/agent.rs` to expose `sys_launch_task_agent`.
- Add `crates/agent-kernel-core/tests/runtime_admission.rs`.
- Add `crates/agent-kernel-core/tests/runtime_admission_errors.rs`.
- Add `crates/agent-kernel/tests/runtime_admission.rs`.
- Modify supervisor formatting, supervisor flow/tests, README, and this plan.

## Task 1: Red Tests

- [x] **Step 1: Add core success tests**

Create `crates/agent-kernel-core/tests/runtime_admission.rs` with tests proving resource-scoped launch admits same-resource task execution and task-scoped launch admits a delegated worker using its delegated capability.

- [x] **Step 2: Add core failure tests**

Create `crates/agent-kernel-core/tests/runtime_admission_errors.rs` with tests for unlaunched enqueue rejection, task-scoped entry mismatch, revoked launch capability blocking ticks, and signal wakeup refusing to requeue a waiter whose launch authority has been revoked.

- [x] **Step 3: Add facade test**

Create `crates/agent-kernel/tests/runtime_admission.rs` proving `sys_launch_task_agent` lets a delegated worker enqueue and dispatch without a root resource capability.

- [x] **Step 4: Verify red**

Run:

```bash
RUSTC="$(rustup which rustc --toolchain nightly)" RUSTDOC="$(rustup which rustdoc --toolchain nightly)" rustup run nightly cargo test -p agent-kernel-core --test runtime_admission --test runtime_admission_errors -p agent-kernel --test runtime_admission
```

Expected: fail because `sys_launch_task_agent`, task-scoped entry fields, and runtime admission errors do not exist.

## Task 2: Core Admission Model

- [x] **Step 1: Add task scope to launch entries**

Add `task: Option<TaskId>` to `AgentEntryRecord`, set it to `None` in existing `launch_agent`, and expose it in tests and event records.

- [x] **Step 2: Implement task-scoped launch**

Add `launch_task_agent(agent, capability, task, kind)` with active-agent, duplicate-entry, assignee, task status, task-scoped `Act` authorization, capacity, event, and entry insertion checks.

- [x] **Step 3: Add admission helper**

Add `ensure_agent_admitted_for_task(agent, task)` to verify resource-scoped or task-scoped entry coverage and live launch capability authority.

- [x] **Step 4: Gate runtime mutation paths**

Call admission checks from enqueue, dispatch, tick, yield, wait, fault, complete, and signal wakeup before mutating runtime state.

- [x] **Step 5: Verify focused core tests**

Run:

```bash
RUSTC="$(rustup which rustc --toolchain nightly)" RUSTDOC="$(rustup which rustdoc --toolchain nightly)" rustup run nightly cargo test -p agent-kernel-core --test runtime_admission --test runtime_admission_errors
```

Expected: pass.

## Task 3: Facade, Runtime, And Docs

- [x] **Step 1: Add facade syscall**

Expose `sys_launch_task_agent(agent, capability, task, kind)` through `agent-kernel`.

- [x] **Step 2: Update supervisor flow**

Launch the target worker after delegation and before enqueueing. Expected new event:

```text
event[18] agent_launched agent=2 resource=1 capability=2 task=1
```

Shift later supervisor event numbers by one.

- [x] **Step 3: Update supervisor formatting**

Format task-scoped launch events with `task=N` and resource-scoped launch events without a task suffix.

- [x] **Step 4: Update README**

Document runtime admission, task-scoped launch, and the new supervisor output.

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

Expected: all commands pass; no_std scan returns no matches; supervisor output includes both owner and worker launch events.

## Self-Review

Spec coverage: this plan covers task-scoped launch, resource-scoped admission, task-scoped admission, runtime gate placement, facade exposure, supervisor output, docs, and verification.

Plan scan: no open-ended implementation notes remain.

Type consistency: `task`, `launch_task_agent`, `sys_launch_task_agent`, `AgentNotLaunched`, and `AgentEntryScopeMismatch` are used consistently.
