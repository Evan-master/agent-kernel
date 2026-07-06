# Kernel Object Namespace V0 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add deterministic, fixed-capacity kernel object namespace entries so agents can name and resolve native kernel objects through workspace capabilities.

**Architecture:** `agent-kernel-core` owns namespace records, object reference validation, capability checks, active-agent checks, and replayable events. `agent-kernel` exposes syscall-style wrappers. Supervisor and QEMU formatters learn the new event kinds without adding POSIX path or host filesystem semantics.

**Tech Stack:** Rust nightly, no_std core and facade crates, fixed-capacity arrays, Cargo workspace tests, QEMU BIOS boot script.

---

## File Structure

- Modify `crates/agent-kernel-core/src/id.rs` for `NamespaceEntryId`.
- Create `crates/agent-kernel-core/src/namespace.rs` for `NamespaceKey`, `NamespaceObject`, and `NamespaceEntryRecord`.
- Create `crates/agent-kernel-core/src/namespace_store.rs` for deterministic bind, resolve, rebind, object validation, and lookup.
- Modify `crates/agent-kernel-core/src/core.rs`, `event.rs`, `error.rs`, and `lib.rs`.
- Modify every `KernelCore<...>` impl header in core source to include trailing `NAMESPACE_ENTRIES` capacity.
- Modify `crates/agent-kernel/src/lib.rs`, `scheduler.rs`, `mailbox.rs`, and `memory.rs` for `AgentKernel<..., NAMESPACE_ENTRIES>`.
- Create `crates/agent-kernel/src/namespace.rs` for namespace syscall wrappers and inspection.
- Add `crates/agent-kernel-core/tests/namespace.rs`.
- Add `crates/agent-kernel-core/tests/namespace_errors.rs`.
- Add `crates/agent-kernel-core/tests/namespace_capacity.rs`.
- Add `crates/agent-kernel/tests/namespace.rs`.
- Modify `crates/agent-supervisor/src/main.rs`, `format.rs`, supervisor tests, QEMU serial output, README, and this design note.

## Task 1: Red Tests

- [x] **Step 1: Add core success-path tests**

Create `crates/agent-kernel-core/tests/namespace.rs` with tests for bind, resolve, and rebind success paths.

- [x] **Step 2: Add core failure-path tests**

Create `crates/agent-kernel-core/tests/namespace_errors.rs` and `crates/agent-kernel-core/tests/namespace_capacity.rs` with tests for non-workspace namespace resource, duplicate key, missing binding, missing referenced object, missing authority, suspended actor ordering, store-full atomicity, bind event-log-full atomicity, resolve event-log-full atomicity, and rebind event-log-full atomicity.

- [x] **Step 3: Add facade namespace test**

Create `crates/agent-kernel/tests/namespace.rs` with one syscall-level test that binds, resolves, rebinds, and inspects `namespace_entries()`.

- [x] **Step 4: Verify red**

Run:

```bash
RUSTC="$(rustup which rustc --toolchain nightly)" RUSTDOC="$(rustup which rustdoc --toolchain nightly)" rustup run nightly cargo test -p agent-kernel-core --test namespace
RUSTC="$(rustup which rustc --toolchain nightly)" RUSTDOC="$(rustup which rustdoc --toolchain nightly)" rustup run nightly cargo test -p agent-kernel-core --test namespace_errors
RUSTC="$(rustup which rustc --toolchain nightly)" RUSTDOC="$(rustup which rustdoc --toolchain nightly)" rustup run nightly cargo test -p agent-kernel-core --test namespace_capacity
RUSTC="$(rustup which rustc --toolchain nightly)" RUSTDOC="$(rustup which rustdoc --toolchain nightly)" rustup run nightly cargo test -p agent-kernel --test namespace
```

Expected: fail because namespace types, events, errors, stores, and syscalls do not exist.

## Task 2: Core Namespace Store

- [x] **Step 1: Add namespace model**

Add `NamespaceEntryId`, `NamespaceKey`, `NamespaceObject`, and `NamespaceEntryRecord`.

- [x] **Step 2: Extend core storage**

Add trailing `NAMESPACE_ENTRIES` capacity, `namespace_entries`, `namespace_entry_len`, and `next_namespace_entry` to `KernelCore`.

- [x] **Step 3: Add errors and events**

Add `NamespaceEntryStoreFull`, `NamespaceEntryNotFound`, and `NamespaceEntryAlreadyExists`, plus `NamespaceEntryBound`, `NamespaceEntryResolved`, and `NamespaceEntryRebound`.

- [x] **Step 4: Implement bind, resolve, rebind, validation, and inspection**

Implement `bind_namespace_entry`, `resolve_namespace_entry`, `rebind_namespace_entry`, `namespace_entries`, and internal lookup helpers. Validate namespace resources as `ResourceKind::Workspace` and referenced objects through their native stores.

- [x] **Step 5: Verify focused core tests**

Run:

```bash
RUSTC="$(rustup which rustc --toolchain nightly)" RUSTDOC="$(rustup which rustdoc --toolchain nightly)" rustup run nightly cargo test -p agent-kernel-core --test namespace
RUSTC="$(rustup which rustc --toolchain nightly)" RUSTDOC="$(rustup which rustdoc --toolchain nightly)" rustup run nightly cargo test -p agent-kernel-core --test namespace_errors
RUSTC="$(rustup which rustc --toolchain nightly)" RUSTDOC="$(rustup which rustdoc --toolchain nightly)" rustup run nightly cargo test -p agent-kernel-core --test namespace_capacity
```

Expected: pass.

## Task 3: Facade, Runtime, And Docs

- [x] **Step 1: Add facade syscalls**

Add `sys_bind_namespace_entry`, `sys_resolve_namespace_entry`, `sys_rebind_namespace_entry`, and `namespace_entries()`.

- [x] **Step 2: Update supervisor flow**

After the memory cell flow, bind the memory cell into the workspace namespace, resolve it, rebind the same key to the created task, and print the resulting events.

- [x] **Step 3: Update QEMU event labels**

Add serial labels for `namespace_entry_bound`, `namespace_entry_resolved`, and `namespace_entry_rebound`.

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

Expected: all commands pass and supervisor output includes one namespace bind, resolve, and rebind event after the memory cell events.

## Self-Review

Spec coverage: the plan covers fixed-capacity storage, workspace scoping, typed object references, object validation, capability authority, active-agent ordering, auditable resolution, revisioned rebinds, atomic failure paths, facade syscalls, runtime formatting, and documentation.

Placeholder scan: no TODO, TBD, or open-ended implementation placeholders remain.

Type consistency: `NamespaceEntryId`, `NamespaceKey`, `NamespaceObject`, `NamespaceEntryRecord`, `NamespaceEntryStoreFull`, `NamespaceEntryNotFound`, `NamespaceEntryAlreadyExists`, `NamespaceEntryBound`, `NamespaceEntryResolved`, `NamespaceEntryRebound`, `bind_namespace_entry`, `resolve_namespace_entry`, `rebind_namespace_entry`, `sys_bind_namespace_entry`, `sys_resolve_namespace_entry`, `sys_rebind_namespace_entry`, and `namespace_entries()` are used consistently.
