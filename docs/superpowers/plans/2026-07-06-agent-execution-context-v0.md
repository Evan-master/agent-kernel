# Agent Execution Context V0 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add one kernel-owned execution context per registered agent and keep it synchronized with scheduler, wait, fault, and task lifecycle transitions.

**Architecture:** `agent-kernel-core` owns the no_std execution context model and updates it from existing task lifecycle operations. Context state uses the existing `AGENTS` capacity and is covered by existing agent/task events instead of adding a new event kind. `agent-kernel` exposes read-only context inspection, while supervisor and docs demonstrate the new runtime boundary.

**Tech Stack:** Rust nightly, no_std core and facade crates, fixed-capacity arrays, Cargo workspace tests, QEMU BIOS boot script.

---

## File Structure

- Create `crates/agent-kernel-core/src/agent_execution.rs` for `AgentExecutionState` and `AgentExecutionContext`.
- Create `crates/agent-kernel-core/src/agent_execution_store.rs` for context inspection and transition helpers.
- Modify `crates/agent-kernel-core/src/core.rs` to store `[AgentExecutionContext; AGENTS]`.
- Modify `crates/agent-kernel-core/src/agent_store.rs` to create idle contexts with agent registration.
- Modify `crates/agent-kernel-core/src/error.rs` to add `ExecutionContextBusy`.
- Modify scheduler, tick, signal, fault, fault policy, and task lifecycle modules to update contexts.
- Create `crates/agent-kernel-core/src/task_completion.rs` for completion, verification, and cancellation transitions.
- Modify `crates/agent-kernel-core/src/lib.rs` to export execution context types and modules.
- Create `crates/agent-kernel-core/tests/agent_execution_context.rs`.
- Create `crates/agent-kernel-core/tests/agent_execution_context_errors.rs`.
- Create `crates/agent-kernel/tests/agent_execution_context.rs`.
- Create `crates/agent-kernel/src/agent.rs` for agent lifecycle and execution context facade methods.
- Modify `crates/agent-kernel/src/lib.rs` to expose `execution_contexts()`.
- Update README current scope and behavior description.

## Task 1: Red Tests

- [x] **Step 1: Add core success tests**

Create `crates/agent-kernel-core/tests/agent_execution_context.rs` with tests for agent registration, dispatch/tick/quantum expiry, yield, wait/wake, fault/recover, and completion.

- [x] **Step 2: Add core error tests**

Create `crates/agent-kernel-core/tests/agent_execution_context_errors.rs` with tests for busy-context dispatch rejection and registration failure atomicity.

- [x] **Step 3: Add facade test**

Create `crates/agent-kernel/tests/agent_execution_context.rs` proving `AgentKernel::execution_contexts()` exposes the same context state after dispatch and completion.

- [x] **Step 4: Verify red**

Run:

```bash
RUSTC="$(rustup which rustc --toolchain nightly)" RUSTDOC="$(rustup which rustdoc --toolchain nightly)" rustup run nightly cargo test -p agent-kernel-core --test agent_execution_context --test agent_execution_context_errors -p agent-kernel --test agent_execution_context
```

Expected: fail because `AgentExecutionContext`, `AgentExecutionState`, `execution_contexts`, and `ExecutionContextBusy` do not exist.

## Task 2: Core Context Model

- [x] **Step 1: Add execution context types**

Add:

```rust
pub enum AgentExecutionState {
    Idle,
    Running,
    Waiting,
    Faulted,
}

pub struct AgentExecutionContext {
    pub agent: AgentId,
    pub state: AgentExecutionState,
    pub task: Option<TaskId>,
    pub run_ticks: u64,
    pub quantum_remaining: u64,
}
```

- [x] **Step 2: Add fixed-capacity context storage**

Add `[AgentExecutionContext; AGENTS]` to `KernelCore` and initialize it with `AgentExecutionContext::empty()`. Use `agent_len` as the context slice length.

- [x] **Step 3: Create contexts during agent registration**

After event capacity validation and before recording `AgentRegistered`, write an idle context at the same index as the new `AgentRecord`.

- [x] **Step 4: Add context inspection and helper transitions**

Add `execution_contexts()`, `execution_context(agent)`, `ensure_execution_context_idle(agent)`, and transition helpers for idle, running, waiting, and faulted states.

- [x] **Step 5: Verify registration tests**

Run:

```bash
RUSTC="$(rustup which rustc --toolchain nightly)" RUSTDOC="$(rustup which rustdoc --toolchain nightly)" rustup run nightly cargo test -p agent-kernel-core --test agent_execution_context register_agent_creates_idle_execution_context
```

Expected: pass.

## Task 3: Runtime Transitions

- [x] **Step 1: Update scheduler dispatch, tick, and yield**

Dispatch requires an idle context and sets it running. Tick updates running context snapshots and clears it on quantum expiry. Yield clears it.

- [x] **Step 2: Update wait and wake**

`wait_task` sets the assignee context to `Waiting`. `emit_signal` clears the woken agent context to `Idle` when it requeues the task.

- [x] **Step 3: Update fault and recovery**

`fault_task` sets the assignee context to `Faulted`. Direct recovery and fault-policy recovery clear it to `Idle`.

- [x] **Step 4: Update task completion and cancellation**

`complete_task` clears the assignee context. `cancel_task` clears the context if the cancelled task is the context's current task.

- [x] **Step 5: Verify core tests**

Run:

```bash
RUSTC="$(rustup which rustc --toolchain nightly)" RUSTDOC="$(rustup which rustdoc --toolchain nightly)" rustup run nightly cargo test -p agent-kernel-core --test agent_execution_context --test agent_execution_context_errors
```

Expected: pass.

## Task 4: Facade, Docs, And Final Verification

- [x] **Step 1: Export facade inspection**

Expose `AgentExecutionContext`, `AgentExecutionState`, and `AgentKernel::execution_contexts()`.

- [x] **Step 2: Update README**

Document execution contexts in current scope and current behavior.

- [x] **Step 3: Verify facade and workspace**

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

Expected: all commands pass; no_std scan returns no matches.

## Self-Review

Spec coverage: this plan covers the execution context model, registration atomicity, dispatch/tick/yield/wait/wake/fault/recover/complete/cancel transitions, busy dispatch rejection, facade inspection, docs, and verification.

Plan scan: no open-ended implementation notes remain.

Type consistency: `AgentExecutionContext`, `AgentExecutionState`, `ExecutionContextBusy`, `execution_contexts`, and the existing task lifecycle method names are used consistently.
