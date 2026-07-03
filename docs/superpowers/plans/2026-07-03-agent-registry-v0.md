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

## Follow-Up: Registered-Agent Authority Tightening

**Goal:** Use the registry as an authority boundary for issuing capabilities.

- [x] Add red tests proving `grant_capability` rejects unregistered agents with
  `AgentNotFound` and no event.
- [x] Add red tests proving `delegate_task` rejects an unregistered target agent
  before deriving a task capability or mutating task delegation fields.
- [x] Check agent registration at the start of `grant_capability`.
- [x] Check agent registration during internal task capability derivation.
- [x] Migrate core and facade tests to explicitly register agents before grants
  and delegation.
- [x] Update design and README docs to state that new root or derived
  capabilities can only be issued to registered agents.
- [x] Verify `cargo test --workspace` with nightly `RUSTC`/`RUSTDOC` shims.

Compatibility note: this step only prevented new authority from being issued to
unknown agents. The follow-up below extends the same registry boundary to
syscall actors.

## Follow-Up: Registered-Actor Syscall Enforcement

**Goal:** Require every kernel operation that acts on behalf of an `AgentId` to
reject unknown actors before authorization, state, queue, or capacity checks.

- [x] Add red tests proving a capability-backed operation by an unregistered
  actor returns `AgentNotFound` before capability mismatch.
- [x] Add red tests proving task accept by an unregistered actor returns
  `AgentNotFound` without changing task state or events.
- [x] Add red tests proving scheduler dispatch by an unregistered actor returns
  `AgentNotFound` without changing task state, queue state, or events.
- [x] Check actor registration in the shared authorization path before resource
  and capability lookup.
- [x] Check actor registration at task lifecycle entrypoints before task lookup
  and task-status validation.
- [x] Check actor registration at scheduler entrypoints before queue-state
  validation.
- [x] Migrate tests that intentionally cover mismatch or queue errors to
  register their wrong actor first.
- [x] Update design and README docs to describe registered-actor syscall
  enforcement and error ordering.

## Follow-Up: Agent Lifecycle Status

**Goal:** Make agent registry status part of the authority boundary.

- [x] Add red tests for `suspend_agent`, `resume_agent`, and `retire_agent`
  status transitions and lifecycle events.
- [x] Add red tests proving suspended and retired agents cannot receive new
  capabilities or use existing authority.
- [x] Add red tests proving a suspended source agent invalidates derived task
  authority through the capability parent chain.
- [x] Extend `AgentStatus` with `Suspended` and `Retired`.
- [x] Add lifecycle event kinds and agent status errors.
- [x] Add core lifecycle APIs and facade syscall wrappers.
- [x] Enforce active-agent status in grant, derive, authorization, task, and
  scheduler entrypoints.
- [x] Update supervisor and QEMU event formatting for lifecycle event labels.
- [x] Update README and design docs with lifecycle authority semantics.

## Self-Review

Spec coverage: the plan covers first-class agent records, fixed capacity, duplicate and capacity errors, facade visibility, boot/supervisor registration, README updates, and verification.

Placeholder scan: no TODO, TBD, or open-ended implementation placeholders remain.

Type consistency: `AgentRecord`, `AgentStatus`, `AgentStoreFull`, `AgentAlreadyExists`, `AgentNotFound`, `AgentSuspended`, `AgentRetired`, `AgentStatusMismatch`, `AgentRegistered`, `AgentSuspended`, `AgentResumed`, `AgentRetired`, `register_agent`, `suspend_agent`, `resume_agent`, `retire_agent`, `sys_register_agent`, `sys_suspend_agent`, `sys_resume_agent`, `sys_retire_agent`, and `agents()` are used consistently.
