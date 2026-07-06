# Task Wait Signal V0 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add deterministic task wait and resource signal wakeup primitives to the native Agent Kernel.

**Architecture:** `agent-kernel-core` owns waiter IDs, signal keys, fixed-capacity waiter records, wait/emit authority checks, task state transitions, run queue wakeup, and replayable events. `agent-kernel` exposes syscall-style wrappers. Supervisor and QEMU format the new labels without introducing host async or POSIX wait semantics.

**Tech Stack:** Rust nightly, no_std core and facade crates, fixed-capacity arrays, Cargo workspace tests, QEMU BIOS boot script.

---

## File Structure

- Modify `crates/agent-kernel-core/src/id.rs` for `WaiterId`.
- Create `crates/agent-kernel-core/src/signal.rs` for `SignalKey`, `WaiterRecord`, and `SignalOutcome`.
- Create `crates/agent-kernel-core/src/signal_event.rs` for wait/signal event construction.
- Create `crates/agent-kernel-core/src/signal_store.rs` for wait and emit behavior.
- Modify `crates/agent-kernel-core/src/core.rs`, `event.rs`, `error.rs`, `lib.rs`, `task.rs`, and scheduler-related helpers.
- Modify every `KernelCore<...>` impl header in core source to include trailing `WAITERS` capacity.
- Modify `crates/agent-kernel/src/lib.rs`, `scheduler.rs`, `fault.rs`, `mailbox.rs`, `memory.rs`, and `namespace.rs` for `AgentKernel<..., WAITERS>`.
- Create `crates/agent-kernel/src/signal.rs` for syscall facade methods.
- Add `crates/agent-kernel-core/tests/task_wait_signal.rs`.
- Add `crates/agent-kernel-core/tests/task_wait_signal_errors.rs`.
- Add `crates/agent-kernel-core/tests/task_wait_signal_queue_errors.rs`.
- Add `crates/agent-kernel/tests/task_wait_signal.rs`.
- Modify `crates/agent-supervisor/src/main.rs`, `format.rs`, supervisor tests, QEMU serial output, README, and this plan.

## Task 1: Red Tests

- [x] **Step 1: Add core success-path tests**

Create `crates/agent-kernel-core/tests/task_wait_signal.rs` with tests for waiting a running task, emitting with no waiter, and emitting a matching signal to wake and enqueue a task.

- [x] **Step 2: Add core failure-path tests**

Create `crates/agent-kernel-core/tests/task_wait_signal_errors.rs` with tests for non-running wait, waiter store full, signal authority failure, and signal event log full. Create `crates/agent-kernel-core/tests/task_wait_signal_queue_errors.rs` with the signal run-queue-full test.

- [x] **Step 3: Add facade signal test**

Create `crates/agent-kernel/tests/task_wait_signal.rs` with a syscall-level wait, emit, redispatch, and complete flow.

- [x] **Step 4: Verify red**

Run:

```bash
RUSTC="$(rustup which rustc --toolchain nightly)" RUSTDOC="$(rustup which rustdoc --toolchain nightly)" rustup run nightly cargo test -p agent-kernel-core --test task_wait_signal
RUSTC="$(rustup which rustc --toolchain nightly)" RUSTDOC="$(rustup which rustdoc --toolchain nightly)" rustup run nightly cargo test -p agent-kernel-core --test task_wait_signal_errors
RUSTC="$(rustup which rustc --toolchain nightly)" RUSTDOC="$(rustup which rustdoc --toolchain nightly)" rustup run nightly cargo test -p agent-kernel --test task_wait_signal
```

Expected: fail because `SignalKey`, `WaiterId`, waiter records, signal events, signal errors, core methods, event fields, and facade syscalls do not exist.

## Task 2: Core Waiter Store

- [x] **Step 1: Add wait/signal model**

Add `WaiterId`, `SignalKey`, `WaiterRecord`, and `SignalOutcome`.

- [x] **Step 2: Extend core storage and events**

Add trailing `WAITERS` capacity, `waiters`, `waiter_len`, and `next_waiter` to `KernelCore`. Add `TaskStatus::Waiting`, `TaskWaiting`, `SignalEmitted`, `TaskWoken`, `waiter`, and `signal`.

- [x] **Step 3: Add wait/signal errors**

Add `WaiterStoreFull`.

- [x] **Step 4: Implement wait and emit**

Implement `wait_task`, `emit_signal`, and `waiters()`. `wait_task` records `TaskWaiting`. `emit_signal` records `SignalEmitted`, and when a waiter matches, marks it inactive, queues the task, and records `TaskWoken`.

- [x] **Step 5: Verify focused core tests**

Run:

```bash
RUSTC="$(rustup which rustc --toolchain nightly)" RUSTDOC="$(rustup which rustdoc --toolchain nightly)" rustup run nightly cargo test -p agent-kernel-core --test task_wait_signal
RUSTC="$(rustup which rustc --toolchain nightly)" RUSTDOC="$(rustup which rustdoc --toolchain nightly)" rustup run nightly cargo test -p agent-kernel-core --test task_wait_signal_errors
```

Expected: pass.

## Task 3: Facade, Runtime, And Docs

- [x] **Step 1: Add facade syscalls**

Add `sys_wait_task`, `sys_emit_signal`, and `waiters()` to `agent-kernel`.

- [x] **Step 2: Update supervisor flow**

After fault recovery redispatches the task, make the assignee wait on a workspace signal, have the owner emit the signal, then redispatch and complete the woken task. The output should include `task_waiting`, `signal_emitted`, and `task_woken`.

- [x] **Step 3: Update QEMU event labels**

Add serial labels for `task_waiting`, `signal_emitted`, and `task_woken`.

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

Expected: all commands pass; the no_std scan returns no matches; supervisor output includes `task_waiting`, `signal_emitted`, `task_woken`, redispatch, and task completion.

## Self-Review

Spec coverage: the plan covers waiter IDs, signal keys, fixed-capacity waiter records, wait authority, signal authority, task waiting state, oldest matching wakeup, run queue integration, event metadata, facade syscalls, runtime formatting, QEMU labels, and documentation.

Placeholder scan: no TODO, TBD, or open-ended implementation placeholders remain.

Type consistency: `WaiterId`, `SignalKey`, `WaiterRecord`, `SignalOutcome`, `WaiterStoreFull`, `TaskWaiting`, `SignalEmitted`, `TaskWoken`, `waiter`, `signal`, `wait_task`, `emit_signal`, `sys_wait_task`, and `sys_emit_signal` are used consistently.
