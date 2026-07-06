# Memory Cell V0 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add deterministic, fixed-capacity kernel memory cells so agents can remember and recall small state through native Agent Kernel primitives.

**Architecture:** `agent-kernel-core` owns memory cell records, value updates, capability checks, active-agent checks, and replayable events. `agent-kernel` exposes syscall-style wrappers. Supervisor and QEMU formatters learn the new event kinds without moving host persistence or allocation into kernel space.

**Tech Stack:** Rust nightly, no_std core and facade crates, fixed-capacity arrays, Cargo workspace tests, QEMU BIOS boot script.

---

## File Structure

- Modify `crates/agent-kernel-core/src/id.rs` for `MemoryCellId`.
- Create `crates/agent-kernel-core/src/memory.rs` for `MemoryValue` and `MemoryCellRecord`.
- Create `crates/agent-kernel-core/src/memory_store.rs` for deterministic create, recall, remember, and lookup.
- Modify `crates/agent-kernel-core/src/core.rs`, `event.rs`, `error.rs`, and `lib.rs`.
- Modify every `KernelCore<...>` impl header in core source to include trailing `MEMORY_CELLS` capacity.
- Modify `crates/agent-kernel/src/lib.rs`, `scheduler.rs`, and `mailbox.rs` for `AgentKernel<..., MEMORY_CELLS>`.
- Create `crates/agent-kernel/src/memory.rs` for memory syscall wrappers and memory cell inspection.
- Add `crates/agent-kernel-core/tests/memory_cell.rs`.
- Add `crates/agent-kernel-core/tests/memory_cell_errors.rs`.
- Add `crates/agent-kernel/tests/memory_cell.rs`.
- Modify `crates/agent-supervisor/src/main.rs`, `format.rs`, supervisor tests, QEMU serial output, README, and this design note.

## Task 1: Red Tests

- [x] **Step 1: Add core success-path tests**

Create `crates/agent-kernel-core/tests/memory_cell.rs` with tests for create, recall, and remember success paths.

- [x] **Step 2: Add core failure-path tests**

Create `crates/agent-kernel-core/tests/memory_cell_errors.rs` with tests for non-memory resource, missing authority, suspended actor ordering, store-full atomicity, create event-log-full atomicity, remember event-log-full atomicity, and recall event-log-full atomicity.

- [x] **Step 3: Add facade memory test**

Create `crates/agent-kernel/tests/memory_cell.rs` with one syscall-level test that creates, recalls, remembers, and inspects `memory_cells()`.

- [x] **Step 4: Verify red**

Run:

```bash
RUSTC="$(rustup which rustc --toolchain nightly)" RUSTDOC="$(rustup which rustdoc --toolchain nightly)" rustup run nightly cargo test -p agent-kernel-core --test memory_cell
RUSTC="$(rustup which rustc --toolchain nightly)" RUSTDOC="$(rustup which rustdoc --toolchain nightly)" rustup run nightly cargo test -p agent-kernel-core --test memory_cell_errors
RUSTC="$(rustup which rustc --toolchain nightly)" RUSTDOC="$(rustup which rustdoc --toolchain nightly)" rustup run nightly cargo test -p agent-kernel --test memory_cell
```

Expected: fail because memory cell types, events, errors, stores, and syscalls do not exist.

## Task 2: Core Memory Store

- [x] **Step 1: Add memory model**

Add `MemoryCellId`, `MemoryValue`, and `MemoryCellRecord`.

- [x] **Step 2: Extend core storage**

Add trailing `MEMORY_CELLS` capacity, `memory_cells`, `memory_cell_len`, and `next_memory_cell` to `KernelCore`.

- [x] **Step 3: Add errors and events**

Add `MemoryCellStoreFull`, `MemoryCellNotFound`, and `ResourceKindMismatch`, plus `MemoryCellCreated`, `MemoryCellRecalled`, and `MemoryCellRemembered`.

- [x] **Step 4: Implement create, recall, remember, and inspection**

Implement `create_memory_cell`, `recall_memory_cell`, `remember_memory_cell`, `memory_cells`, and internal memory cell lookup helpers.

- [x] **Step 5: Verify focused core tests**

Run:

```bash
RUSTC="$(rustup which rustc --toolchain nightly)" RUSTDOC="$(rustup which rustdoc --toolchain nightly)" rustup run nightly cargo test -p agent-kernel-core --test memory_cell
RUSTC="$(rustup which rustc --toolchain nightly)" RUSTDOC="$(rustup which rustdoc --toolchain nightly)" rustup run nightly cargo test -p agent-kernel-core --test memory_cell_errors
```

Expected: pass.

## Task 3: Facade, Runtime, And Docs

- [x] **Step 1: Add facade syscalls**

Add `sys_create_memory_cell`, `sys_recall_memory_cell`, `sys_remember_memory_cell`, and `memory_cells()`.

- [x] **Step 2: Update supervisor flow**

After the mailbox acknowledgement flow, register a memory resource, grant observe/act authority, create one memory cell, recall it, remember a new value, and print the resulting events.

- [x] **Step 3: Update QEMU event labels**

Add serial labels for `memory_cell_created`, `memory_cell_recalled`, and `memory_cell_remembered`.

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
git status --short --branch
```

Expected: all commands pass and supervisor output includes one memory cell create, recall, and remember event after mailbox acknowledgement.

## Self-Review

Spec coverage: the plan covers fixed-capacity storage, capability authority, active-agent ordering, auditable recall, revisioned writes, atomic failure paths, facade syscalls, runtime formatting, and documentation.

Placeholder scan: no TODO, TBD, or open-ended implementation placeholders remain.

Type consistency: `MemoryCellId`, `MemoryValue`, `MemoryCellRecord`, `MemoryCellStoreFull`, `MemoryCellNotFound`, `ResourceKindMismatch`, `MemoryCellCreated`, `MemoryCellRecalled`, `MemoryCellRemembered`, `create_memory_cell`, `recall_memory_cell`, `remember_memory_cell`, `sys_create_memory_cell`, `sys_recall_memory_cell`, `sys_remember_memory_cell`, and `memory_cells()` are used consistently.
