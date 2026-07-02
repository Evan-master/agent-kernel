# Agent Kernel V0 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build the first Agent Kernel prototype as a Rust workspace with a no_std-friendly core model, a no_std kernel facade, and a host supervisor simulator.

**Architecture:** The prototype starts with AgentOS-native primitives instead of POSIX primitives: resources, capabilities, actions, observations, verification events, and checkpoints. The kernel core is pure Rust and fixed-capacity so it can compile without heap allocation. The supervisor is a normal host binary that exercises the kernel model before there is bootable hardware support.

**Tech Stack:** Rust 2021, Cargo workspace, no_std-compatible libraries, host-side integration tests.

---

### Task 1: Workspace And Red Tests

**Files:**
- Create: `Cargo.toml`
- Create: `.gitignore`
- Create: `README.md`
- Create: `crates/agent-kernel-core/Cargo.toml`
- Create: `crates/agent-kernel-core/src/lib.rs`
- Create: `crates/agent-kernel-core/tests/kernel_core.rs`

- [ ] **Step 1: Create the Cargo workspace**

Create a workspace with three crates: `agent-kernel-core`, `agent-kernel`, and `agent-supervisor`.

- [ ] **Step 2: Write failing core tests**

Add tests proving the intended kernel model:
- a resource can be registered and observed through a granted capability
- an unauthorized action is denied
- revoking a capability prevents future authorization
- checkpoints and rollbacks are recorded in order

- [ ] **Step 3: Run red verification**

Run: `cargo test -p agent-kernel-core`

Expected: failure caused by missing core API, not by Cargo workspace errors.

### Task 2: Core Kernel Model

**Files:**
- Modify: `crates/agent-kernel-core/src/lib.rs`
- Modify: `crates/agent-kernel-core/tests/kernel_core.rs`

- [ ] **Step 1: Implement identifiers and enums**

Implement typed IDs for agents, resources, capabilities, tasks, and checkpoints. Implement resource kinds, operation kinds, kernel errors, action status, and event kinds.

- [ ] **Step 2: Implement fixed-capacity stores**

Implement `KernelCore<const RESOURCES: usize, const CAPS: usize, const EVENTS: usize>` using arrays of `Option<T>` so it remains no_std-friendly.

- [ ] **Step 3: Implement capability authorization**

Implement `register_resource`, `grant_capability`, `authorize`, `revoke_capability`, `checkpoint`, `rollback`, and `events`.

- [ ] **Step 4: Run green verification**

Run: `cargo test -p agent-kernel-core`

Expected: all core tests pass.

### Task 3: Kernel Facade

**Files:**
- Create: `crates/agent-kernel/Cargo.toml`
- Create: `crates/agent-kernel/src/lib.rs`
- Create: `crates/agent-kernel/tests/kernel_facade.rs`

- [ ] **Step 1: Write facade tests**

Add tests showing `AgentKernel` starts with an empty event log, can perform an observe syscall when supplied with a valid capability, and requires explicit checkpoint/rollback capabilities before recording checkpoint or rollback events.

- [ ] **Step 2: Run red verification**

Run: `cargo test -p agent-kernel`

Expected: failure caused by missing facade API.

- [ ] **Step 3: Implement facade**

Wrap `KernelCore` in `AgentKernel` and expose syscall-style methods: `sys_register_resource`, `sys_grant`, `sys_observe`, `sys_checkpoint`, and `sys_rollback`. `sys_checkpoint` and `sys_rollback` must accept a capability ID and authorize it before recording events.

- [ ] **Step 4: Run green verification**

Run: `cargo test -p agent-kernel`

Expected: all facade tests pass.

### Task 4: Supervisor Simulator

**Files:**
- Create: `crates/agent-supervisor/Cargo.toml`
- Create: `crates/agent-supervisor/src/main.rs`

- [ ] **Step 1: Implement host simulator**

Create a binary that boots the kernel facade in-process, registers a workspace resource, grants observe/checkpoint/rollback capabilities to an agent, performs a minimal task flow, and prints the event log.

- [ ] **Step 2: Run simulator**

Run: `cargo run -p agent-supervisor`

Expected: output lists observation, checkpoint, and rollback events.

### Task 5: Documentation And Publishing

**Files:**
- Modify: `README.md`

- [ ] **Step 1: Document the prototype**

Describe what this v0 is, what it deliberately is not, and how to run tests and the simulator.

- [ ] **Step 2: Verify full workspace**

Run: `cargo test --workspace`
Run: `cargo run -p agent-supervisor`

- [ ] **Step 3: Commit and push**

Commit the project, create a private GitHub repository with `gh repo create agent-kernel --private --source=. --remote=origin --push`, and confirm the remote URL.
