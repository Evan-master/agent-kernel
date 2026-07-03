# Agent Registry V0 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a deterministic, fixed-capacity kernel registry for first-class agent records.

**Architecture:** `agent-kernel-core` gains `AgentRecord`, `AgentStatus`, and an agent registry store. `agent-kernel` exposes syscall-style registration and read-only inspection. Boot and supervisor flows explicitly register agents before later kernel operations.

**Tech Stack:** Rust nightly, no_std core and facade crates, fixed-capacity arrays, Cargo workspace tests, QEMU BIOS boot script.

---

## File Structure

- Create `crates/agent-kernel-core/src/agent.rs` for `AgentRecord` and `AgentStatus`.
- Create `crates/agent-kernel-core/src/agent_store.rs` for deterministic registration and lookup.
- Modify `crates/agent-kernel-core/src/core.rs`, `event.rs`, `error.rs`, and `lib.rs`.
- Modify every `KernelCore<...>` impl and test alias to include the leading `AGENTS` capacity.
- Modify `crates/agent-kernel/src/lib.rs` and `scheduler.rs` for `AgentKernel<AGENTS, ...>`.
- Modify `crates/agent-kernel-boot/src/lib.rs`, boot tests, and x86 serial output for bootstrap agent registration.
- Modify `crates/agent-supervisor/src/main.rs`, supervisor tests, and `README.md` for registered owner and target agents.
- Add design note `docs/superpowers/specs/2026-07-03-agent-registry-v0-design.md`.

## Task 1: Red Tests

**Files:**
- Create: `crates/agent-kernel-core/tests/agent_registry.rs`
- Create: `crates/agent-kernel/tests/agent_registry.rs`

- [x] **Step 1: Add core registry tests**

Cover successful registration, duplicate rejection, store capacity failure, and event-log capacity failure.

- [x] **Step 2: Add facade registry test**

Cover `sys_register_agent` and `agents()` visibility.

- [x] **Step 3: Verify red**

Run:

```bash
RUSTC="$(rustup which rustc --toolchain nightly)" RUSTDOC="$(rustup which rustdoc --toolchain nightly)" rustup run nightly cargo test -p agent-kernel-core --test agent_registry
RUSTC="$(rustup which rustc --toolchain nightly)" RUSTDOC="$(rustup which rustdoc --toolchain nightly)" rustup run nightly cargo test -p agent-kernel --test agent_registry
```

Expected: fail because `AgentStatus`, `AgentRegistered`, registry errors, and registration APIs do not exist yet.

## Task 2: Core Registry

**Files:**
- Create: `crates/agent-kernel-core/src/agent.rs`
- Create: `crates/agent-kernel-core/src/agent_store.rs`
- Modify: `crates/agent-kernel-core/src/core.rs`
- Modify: `crates/agent-kernel-core/src/error.rs`
- Modify: `crates/agent-kernel-core/src/event.rs`
- Modify: `crates/agent-kernel-core/src/lib.rs`

- [x] **Step 1: Add agent record model**

Add `AgentStatus::Active` and `AgentRecord { id, status }`.

- [x] **Step 2: Add registry errors and event kind**

Add `AgentStoreFull`, `AgentAlreadyExists`, `AgentNotFound`, and `EventKind::AgentRegistered`.

- [x] **Step 3: Extend `KernelCore`**

Add leading `AGENTS` capacity, `[AgentRecord; AGENTS]`, and `agent_len`.

- [x] **Step 4: Implement `register_agent`**

Check duplicate id, store capacity, and event capacity before mutating. Store `AgentStatus::Active` and emit `AgentRegistered` with `target_agent: Some(agent)`.

- [x] **Step 5: Verify focused tests**

Run:

```bash
RUSTC="$(rustup which rustc --toolchain nightly)" RUSTDOC="$(rustup which rustdoc --toolchain nightly)" rustup run nightly cargo test -p agent-kernel-core --test agent_registry
```

Expected: pass.

## Task 3: Facade And Arity Propagation

**Files:**
- Modify: `crates/agent-kernel/src/lib.rs`
- Modify: `crates/agent-kernel/src/scheduler.rs`
- Modify: core and facade tests using `KernelCore<...>` or `AgentKernel<...>`.

- [x] **Step 1: Add facade APIs**

Add `sys_register_agent(agent)` and `agents()`.

- [x] **Step 2: Propagate `AGENTS` capacity**

Insert `AGENTS` as the first const generic in core/facade types and fixed-capacity aliases.

- [x] **Step 3: Verify workspace tests**

Run:

```bash
rustup run nightly cargo fmt --check
RUSTC="$(rustup which rustc --toolchain nightly)" RUSTDOC="$(rustup which rustdoc --toolchain nightly)" rustup run nightly cargo test --workspace
```

Expected: pass.

## Task 4: Runtime And Docs

**Files:**
- Modify: `crates/agent-kernel-boot/src/lib.rs`
- Modify: `crates/agent-kernel-boot/tests/boot_flow.rs`
- Modify: `crates/agent-kernel-x86_64/src/main.rs`
- Modify: `crates/agent-supervisor/src/main.rs`
- Modify: `crates/agent-supervisor/tests/supervisor_flow.rs`
- Modify: `README.md`

- [x] **Step 1: Register runtime agents**

Boot registers the bootstrap agent. Supervisor registers owner and target agents before resource setup.

- [x] **Step 2: Update output formatting**

Print `agent_registered` in supervisor and QEMU serial output.

- [x] **Step 3: Update docs**

README current behavior, QEMU expected output, and supervisor expected output include agent registration.

- [x] **Step 4: Final verification**

Run:

```bash
rustup run nightly cargo fmt --check
RUSTC="$(rustup which rustc --toolchain nightly)" RUSTDOC="$(rustup which rustdoc --toolchain nightly)" rustup run nightly cargo test --workspace
RUSTC="$(rustup which rustc --toolchain nightly)" RUSTDOC="$(rustup which rustdoc --toolchain nightly)" rustup run nightly cargo run -p agent-supervisor
scripts/run-qemu.sh
git diff --check
git status --short --branch
```

Expected: all commands pass, supervisor shows two `agent_registered` events, QEMU shows one `agent_registered` event, and status only contains intended files before commit.

## Self-Review

Spec coverage: the plan covers first-class agent records, fixed capacity, duplicate and capacity errors, facade visibility, boot/supervisor registration, README updates, and verification.

Placeholder scan: no TODO, TBD, or open-ended implementation placeholders remain.

Type consistency: `AgentRecord`, `AgentStatus`, `AgentStoreFull`, `AgentAlreadyExists`, `AgentNotFound`, `AgentRegistered`, `register_agent`, `sys_register_agent`, and `agents()` are used consistently.
