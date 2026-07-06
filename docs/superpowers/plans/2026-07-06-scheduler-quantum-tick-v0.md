# Scheduler Quantum Tick V0 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add deterministic task quantum and tick accounting so Agent Kernel tasks can be advanced, audited, and preempted back into the run queue without host timers or threads.

**Architecture:** `agent-kernel-core` owns task tick counters, explicit quantum dispatch, tick state transitions, fixed-capacity requeue behavior, and replayable scheduler events. `agent-kernel` exposes syscall-style wrappers. Supervisor and QEMU learn the new event labels while all timing remains deterministic and externally driven.

**Tech Stack:** Rust nightly, no_std core and facade crates, fixed-capacity arrays, Cargo workspace tests, QEMU BIOS boot script.

---

## File Structure

- Modify `crates/agent-kernel-core/src/task.rs` for `run_ticks` and `quantum_remaining`.
- Modify `crates/agent-kernel-core/src/scheduler.rs` for `dispatch_next_with_quantum`, default quantum dispatch, and scheduler event fields.
- Create `crates/agent-kernel-core/src/scheduler_tick.rs` for deterministic task tick advancement and quantum expiry.
- Modify `crates/agent-kernel-core/src/error.rs`, `event.rs`, and all `Event` initializers for new error and event metadata fields.
- Add `crates/agent-kernel-core/tests/scheduler_quantum.rs`.
- Add `crates/agent-kernel-core/tests/scheduler_quantum_errors.rs`.
- Modify `crates/agent-kernel/src/scheduler.rs` for `sys_dispatch_next_with_quantum` and `sys_tick_task`.
- Add `crates/agent-kernel/tests/scheduler_quantum.rs`.
- Modify `crates/agent-supervisor/src/main.rs`, `format.rs`, supervisor tests, QEMU serial output, README, and this design note.

## Task 1: Red Tests

- [x] **Step 1: Add core success-path tests**

Create `crates/agent-kernel-core/tests/scheduler_quantum.rs` with tests for default quantum dispatch, explicit quantum dispatch, non-expiring tick, and quantum expiry requeue.

- [x] **Step 2: Add core failure-path tests**

Create `crates/agent-kernel-core/tests/scheduler_quantum_errors.rs` with tests for zero quantum dispatch, ticking an accepted task, ticking another agent's running task, tick event-log-full atomicity, and quantum-expiry run-queue-full atomicity.

- [x] **Step 3: Add facade scheduler quantum test**

Create `crates/agent-kernel/tests/scheduler_quantum.rs` with one syscall-level test that dispatches with quantum `2`, ticks once, ticks again, and inspects task counters plus run queue state.

- [x] **Step 4: Verify red**

Run:

```bash
RUSTC="$(rustup which rustc --toolchain nightly)" RUSTDOC="$(rustup which rustdoc --toolchain nightly)" rustup run nightly cargo test -p agent-kernel-core --test scheduler_quantum
RUSTC="$(rustup which rustc --toolchain nightly)" RUSTDOC="$(rustup which rustdoc --toolchain nightly)" rustup run nightly cargo test -p agent-kernel-core --test scheduler_quantum_errors
RUSTC="$(rustup which rustc --toolchain nightly)" RUSTDOC="$(rustup which rustdoc --toolchain nightly)" rustup run nightly cargo test -p agent-kernel --test scheduler_quantum
```

Expected: fail because task quantum fields, `TaskTicked`, `TaskQuantumExpired`, `TaskQuantumInvalid`, `dispatch_next_with_quantum`, `tick_task`, and facade syscalls do not exist.

## Task 2: Core Scheduler Quantum

- [x] **Step 1: Add task counters**

Add `run_ticks: u64` and `quantum_remaining: u64` to `Task`, initialize them to `0`, and set new task records to `0`.

- [x] **Step 2: Add errors and events**

Add `TaskQuantumInvalid`, `TaskTicked`, and `TaskQuantumExpired`. Add `task_ticks: Option<u64>` and `task_quantum: Option<u64>` to `Event`, defaulting to `None` in non-scheduler events.

- [x] **Step 3: Implement explicit quantum dispatch**

Implement `dispatch_next_with_quantum(agent, quantum)` and make existing `dispatch_next(agent)` delegate to it with quantum `1`. Dispatch validates nonzero quantum, pops the oldest runnable task, marks it `Running`, sets `quantum_remaining`, records `TaskDispatched`, and includes `task_quantum`.

- [x] **Step 4: Implement deterministic ticking**

Implement `tick_task(agent, task)`. A non-expiring tick increments `run_ticks`, decrements `quantum_remaining`, and records `TaskTicked`. An expiring tick checks queue and event capacity before mutation, increments `run_ticks`, sets `quantum_remaining` to `0`, moves the task to `Accepted`, requeues it at the back, and records `TaskQuantumExpired`.

- [x] **Step 5: Verify focused core tests**

Run:

```bash
RUSTC="$(rustup which rustc --toolchain nightly)" RUSTDOC="$(rustup which rustdoc --toolchain nightly)" rustup run nightly cargo test -p agent-kernel-core --test scheduler_quantum
RUSTC="$(rustup which rustc --toolchain nightly)" RUSTDOC="$(rustup which rustdoc --toolchain nightly)" rustup run nightly cargo test -p agent-kernel-core --test scheduler_quantum_errors
```

Expected: pass.

## Task 3: Facade, Runtime, And Docs

- [x] **Step 1: Add facade syscalls**

Add `sys_dispatch_next_with_quantum(agent, quantum)` and `sys_tick_task(agent, task)` to `crates/agent-kernel/src/scheduler.rs`.

- [x] **Step 2: Update supervisor flow**

Dispatch the delegated task with quantum `2`, tick once, tick again to expire and requeue, dispatch it again with quantum `2`, then complete and verify it. The supervisor output should include `task_ticked` and `task_quantum_expired`.

- [x] **Step 3: Update QEMU event labels**

Add serial labels for `task_ticked` and `task_quantum_expired`.

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

Expected: all commands pass; the no_std scan returns no matches; supervisor output includes `task_ticked` and `task_quantum_expired` between task dispatch and final task completion.

## Self-Review

Spec coverage: the plan covers deterministic quantum dispatch, default quantum compatibility, tick accounting, preemption by quantum expiry, fixed-capacity queue behavior, atomic failure paths, facade syscalls, runtime formatting, QEMU labels, and documentation.

Placeholder scan: no TODO, TBD, or open-ended implementation placeholders remain.

Type consistency: `run_ticks`, `quantum_remaining`, `TaskQuantumInvalid`, `TaskTicked`, `TaskQuantumExpired`, `task_ticks`, `task_quantum`, `dispatch_next_with_quantum`, `tick_task`, `sys_dispatch_next_with_quantum`, and `sys_tick_task` are used consistently.
