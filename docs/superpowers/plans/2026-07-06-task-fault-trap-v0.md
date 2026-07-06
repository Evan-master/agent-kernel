# Task Fault Trap V0 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add deterministic task fault traps so running agent tasks can fail, be recorded by the kernel, and be recovered through explicit rollback authority.

**Architecture:** `agent-kernel-core` owns fault IDs, fixed-capacity fault records, task fault status, atomic task/fault mutations, and replayable fault events. `agent-kernel` exposes syscall-style wrappers. Supervisor and QEMU format the new event labels without turning faults into host panics or POSIX signals.

**Tech Stack:** Rust nightly, no_std core and facade crates, fixed-capacity arrays, Cargo workspace tests, QEMU BIOS boot script.

---

## File Structure

- Modify `crates/agent-kernel-core/src/id.rs` for `FaultId`.
- Create `crates/agent-kernel-core/src/fault.rs` for `FaultKind` and `FaultRecord`.
- Create `crates/agent-kernel-core/src/fault_store.rs` for deterministic fault and recovery behavior.
- Modify `crates/agent-kernel-core/src/task.rs` for `TaskStatus::Faulted` and `last_fault`.
- Modify `crates/agent-kernel-core/src/core.rs`, `event.rs`, `error.rs`, and `lib.rs`.
- Modify every `KernelCore<...>` impl header in core source to include trailing `FAULTS` capacity.
- Modify `crates/agent-kernel/src/lib.rs`, `scheduler.rs`, `mailbox.rs`, `memory.rs`, and `namespace.rs` for `AgentKernel<..., FAULTS>`.
- Create `crates/agent-kernel/src/fault.rs` for fault syscall wrappers and inspection.
- Add `crates/agent-kernel-core/tests/task_fault.rs`.
- Add `crates/agent-kernel-core/tests/task_fault_errors.rs`.
- Add `crates/agent-kernel/tests/task_fault.rs`.
- Modify `crates/agent-supervisor/src/main.rs`, `format.rs`, supervisor tests, QEMU serial output, README, and this design note.

## Task 1: Red Tests

- [x] **Step 1: Add core success-path tests**

Create `crates/agent-kernel-core/tests/task_fault.rs` with tests for faulting a running task and recovering a faulted task.

- [x] **Step 2: Add core failure-path tests**

Create `crates/agent-kernel-core/tests/task_fault_errors.rs` with tests for faulting non-running tasks, wrong-agent faults, fault-store-full atomicity, fault event-log-full atomicity, recovery missing rollback authority, and recovery event-log-full atomicity.

- [x] **Step 3: Add facade fault test**

Create `crates/agent-kernel/tests/task_fault.rs` with one syscall-level test that dispatches, faults, inspects faults, recovers, requeues, redispatches, and completes a task.

- [x] **Step 4: Verify red**

Run:

```bash
RUSTC="$(rustup which rustc --toolchain nightly)" RUSTDOC="$(rustup which rustdoc --toolchain nightly)" rustup run nightly cargo test -p agent-kernel-core --test task_fault
RUSTC="$(rustup which rustc --toolchain nightly)" RUSTDOC="$(rustup which rustdoc --toolchain nightly)" rustup run nightly cargo test -p agent-kernel-core --test task_fault_errors
RUSTC="$(rustup which rustc --toolchain nightly)" RUSTDOC="$(rustup which rustdoc --toolchain nightly)" rustup run nightly cargo test -p agent-kernel --test task_fault
```

Expected: fail because `FaultId`, `FaultKind`, fault records, task fault status, events, errors, core methods, and facade syscalls do not exist.

## Task 2: Core Fault Store

- [x] **Step 1: Add fault model**

Add `FaultId`, `FaultKind`, and `FaultRecord`.

- [x] **Step 2: Extend core and task storage**

Add trailing `FAULTS` capacity, `faults`, `fault_len`, and `next_fault` to `KernelCore`. Add `TaskStatus::Faulted` and `last_fault: Option<FaultId>` to `Task`.

- [x] **Step 3: Add errors and events**

Add `FaultStoreFull`, `TaskFaulted`, and `TaskFaultRecovered`. Add `fault: Option<FaultId>`, `fault_kind: Option<FaultKind>`, and `fault_detail: Option<u64>` to `Event`, defaulting to `None` in non-fault events.

- [x] **Step 4: Implement faulting and recovery**

Implement `fault_task`, `recover_faulted_task`, and `faults()`. Faulting requires assigned running task ownership. Recovery requires `Operation::Rollback` authority. Both operations are atomic on capacity and event-log failures.

- [x] **Step 5: Verify focused core tests**

Run:

```bash
RUSTC="$(rustup which rustc --toolchain nightly)" RUSTDOC="$(rustup which rustdoc --toolchain nightly)" rustup run nightly cargo test -p agent-kernel-core --test task_fault
RUSTC="$(rustup which rustc --toolchain nightly)" RUSTDOC="$(rustup which rustdoc --toolchain nightly)" rustup run nightly cargo test -p agent-kernel-core --test task_fault_errors
```

Expected: pass.

## Task 3: Facade, Runtime, And Docs

- [x] **Step 1: Add facade syscalls**

Add `sys_fault_task`, `sys_recover_faulted_task`, and `faults()` to `agent-kernel`.

- [x] **Step 2: Update supervisor flow**

After redispatching the expired task, fault it, recover it with the owner rollback capability, requeue and redispatch it, then complete and verify it. The output should include `task_faulted` and `task_fault_recovered`.

- [x] **Step 3: Update QEMU event labels**

Add serial labels for `task_faulted` and `task_fault_recovered`.

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

Expected: all commands pass; the no_std scan returns no matches; supervisor output includes `task_faulted` and `task_fault_recovered` before task completion.

## Self-Review

Spec coverage: the plan covers fault IDs, fixed-capacity records, task fault status, fault event metadata, assigned-agent trap authority, rollback-authorized recovery, atomic failure paths, facade syscalls, runtime formatting, QEMU labels, and documentation.

Placeholder scan: no TODO, TBD, or open-ended implementation placeholders remain.

Type consistency: `FaultId`, `FaultKind`, `FaultRecord`, `FaultStoreFull`, `TaskFaulted`, `TaskFaultRecovered`, `last_fault`, `fault`, `fault_kind`, `fault_detail`, `fault_task`, `recover_faulted_task`, `sys_fault_task`, and `sys_recover_faulted_task` are used consistently.
