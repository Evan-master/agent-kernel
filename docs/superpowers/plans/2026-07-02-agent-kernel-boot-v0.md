# Agent Kernel Boot V0 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a no_std boot boundary crate that seeds the Agent Kernel with a deterministic bootstrap agent handoff flow.

**Architecture:** `agent-kernel-boot` is a no_std library crate, not a host runner and not a bootloader dependency. It owns boot phases, boot configuration, and a deterministic `BootedKernel` wrapper that initializes `AgentKernel` through syscall-style methods. Real QEMU/UEFI boot is deferred until emulator/toolchain support is available, but this crate gives that future entrypoint a stable kernel-native handoff contract.

**Tech Stack:** Rust 2021, Cargo workspace, no_std-compatible library, host-side integration tests.

---

### Task 1: Boot Crate Red Tests

**Files:**
- Modify: `Cargo.toml`
- Create: `crates/agent-kernel-boot/Cargo.toml`
- Create: `crates/agent-kernel-boot/src/lib.rs`
- Create: `crates/agent-kernel-boot/tests/boot_flow.rs`

- [ ] **Step 1: Add boot crate scaffold**

Add `crates/agent-kernel-boot` as a workspace member with dependencies on `agent-kernel` and `agent-kernel-core`.

- [ ] **Step 2: Write failing boot tests**

Add tests proving:
- `BootedKernel::boot(BootConfig::default())` records the phases `EnteredKernel`, `KernelInitialized`, and `SupervisorHandoffReady`.
- boot creates the event sequence observation → action → verification using the configured bootstrap action.

- [ ] **Step 3: Run red verification**

Run: `cargo test -p agent-kernel-boot`

Expected: failure caused by missing boot API, not by workspace errors.

### Task 2: no_std Boot Protocol

**Files:**
- Modify: `crates/agent-kernel-boot/src/lib.rs`

- [ ] **Step 1: Implement boot types**

Implement `BootPhase`, `BootConfig`, `BootReport`, and `BootedKernel`.

- [ ] **Step 2: Implement bootstrap flow**

`BootedKernel::boot` must:
- create an `AgentKernel`,
- register a bootstrap resource,
- grant observe/act/verify operations to the bootstrap agent,
- record observation, action, and verification events,
- return a report with the boot phases and identifiers.

- [ ] **Step 3: Run green verification**

Run: `cargo test -p agent-kernel-boot`

Expected: boot crate tests pass.

### Task 3: Documentation And Workspace Verification

**Files:**
- Modify: `README.md`

- [ ] **Step 1: Document boot crate scope**

Document that boot v0 is a no_std handoff boundary and that QEMU boot is not yet wired because QEMU is not installed in this local environment.

- [ ] **Step 2: Run workspace verification**

Run:

```bash
cargo fmt --check
cargo test --workspace
cargo run -p agent-supervisor
cargo build -p agent-kernel-boot
RUSTC=$(rustup which rustc) rustup run stable cargo build -p agent-kernel-boot --target x86_64-unknown-none
```

- [ ] **Step 3: Commit and push**

Commit with `feat: add boot handoff crate` and push to `origin/main`.
