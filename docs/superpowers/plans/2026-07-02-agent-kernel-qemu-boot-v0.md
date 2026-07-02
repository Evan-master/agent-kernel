# Agent Kernel QEMU Boot V0 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build and run a minimal x86_64 Agent Kernel image in QEMU with deterministic serial boot output.

**Architecture:** Add an architecture-specific `agent-kernel-x86_64` no_std binary crate that uses `bootloader_api` for the real boot entry and calls the existing `agent-kernel-boot` handoff flow. Add a host-side `agent-kernel-image` tool that wraps `bootloader::DiskImageBuilder` to create a BIOS disk image. Shell scripts orchestrate rustup Cargo, image creation, and QEMU execution without moving host-specific logic into kernel crates.

**Tech Stack:** Rust nightly via rustup, bootloader 0.11, bootloader_api 0.11, QEMU 11.0.2, x86_64-unknown-none.

---

### Task 1: Toolchain And Image Tool Red Tests

**Files:**
- Create: `rust-toolchain.toml`
- Modify: `Cargo.toml`
- Create: `crates/agent-kernel-image/Cargo.toml`
- Create: `crates/agent-kernel-image/src/lib.rs`
- Create: `crates/agent-kernel-image/src/main.rs`
- Create: `crates/agent-kernel-image/tests/image_tool.rs`

- [ ] **Step 1: Pin Rust toolchain**

Use the installed `nightly-aarch64-apple-darwin` toolchain because `bootloader` 0.11 builds its internal BIOS stages with Cargo `-Z` flags.

- [ ] **Step 2: Write failing image tool tests**

Tests must prove:
- `BuildPaths::new("kernel.elf", "agent.img")` preserves kernel and image paths.
- `qemu_bios_args("agent.img")` includes `-drive`, `format=raw,file=agent.img`, `-serial`, `stdio`, and `isa-debug-exit`.

- [ ] **Step 3: Run red verification**

Run: `rustup run nightly cargo test -p agent-kernel-image`

Expected: failure caused by missing image tool API.

### Task 2: Image Builder And Kernel Entry

**Files:**
- Create: `crates/agent-kernel-x86_64/Cargo.toml`
- Create: `crates/agent-kernel-x86_64/src/main.rs`
- Modify: `crates/agent-kernel-image/src/lib.rs`
- Modify: `crates/agent-kernel-image/src/main.rs`

- [ ] **Step 1: Implement image tool**

Use `bootloader::DiskImageBuilder::new(kernel_path).create_bios_image(image_path)` in the host tool. Keep it out of no_std crates.

- [ ] **Step 2: Implement x86_64 kernel entry**

Use `bootloader_api::entry_point!` and call `BootedKernel::boot(BootConfig::default())`. Print serial lines for observation, action, verification, and supervisor handoff. Use `isa-debug-exit` to terminate QEMU with a known success status.

- [ ] **Step 3: Run green tests**

Run: `rustup run nightly cargo test -p agent-kernel-image`

Expected: image tool tests pass.

### Task 3: Scripts, QEMU Run, And Docs

**Files:**
- Create: `scripts/build-qemu-image.sh`
- Create: `scripts/run-qemu.sh`
- Modify: `README.md`

- [ ] **Step 1: Add scripts**

`build-qemu-image.sh` must build the x86_64 kernel and create `target/agent-kernel-x86_64-bios.img`.

`run-qemu.sh` must build the image if needed, run QEMU with serial stdio and isa-debug-exit, and treat QEMU exit code 33 as success.

- [ ] **Step 2: Run QEMU**

Run: `scripts/run-qemu.sh`

Expected serial output contains:

```text
AGENT_KERNEL_QEMU_BOOT_OK
event[1] observation
event[2] action
event[3] verification
SUPERVISOR_HANDOFF_READY
```

- [ ] **Step 3: Full verification**

Run:

```bash
rustup run nightly cargo fmt --check
rustup run nightly cargo test --workspace
scripts/run-qemu.sh
```

- [ ] **Step 4: Commit and push**

Commit with `feat: boot agent kernel in qemu` and push to `origin/main`.
