# Fault Handler Routing V0 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add deterministic fault handler routing so a faulted task can be routed to a registered handler agent through native kernel IPC.

**Architecture:** `agent-kernel-core` owns handler IDs, fixed-capacity handler records, install/route authority checks, atomic message routing, and replayable route events. `agent-kernel` exposes syscall-style wrappers. Supervisor and QEMU format the new event labels without turning handlers into host callbacks.

**Tech Stack:** Rust nightly, no_std core and facade crates, fixed-capacity arrays, Cargo workspace tests, QEMU BIOS boot script.

---

## File Structure

- Modify `crates/agent-kernel-core/src/id.rs` for `FaultHandlerId`.
- Create `crates/agent-kernel-core/src/fault_handler.rs` for `FaultHandlerRecord`.
- Create `crates/agent-kernel-core/src/fault_handler_store.rs` for install and route behavior.
- Modify `crates/agent-kernel-core/src/core.rs`, `event.rs`, `error.rs`, `lib.rs`, `message.rs`, and `mailbox_store.rs`.
- Modify every `KernelCore<...>` impl header in core source to include trailing `FAULT_HANDLERS` capacity.
- Modify `crates/agent-kernel/src/lib.rs`, `fault.rs`, `mailbox.rs`, `memory.rs`, `namespace.rs`, and `scheduler.rs` for `AgentKernel<..., FAULT_HANDLERS>`.
- Add `crates/agent-kernel-core/tests/fault_handler.rs`.
- Add `crates/agent-kernel-core/tests/fault_handler_errors.rs`.
- Add `crates/agent-kernel/tests/fault_handler.rs`.
- Modify `crates/agent-supervisor/src/main.rs`, `format.rs`, supervisor tests, QEMU serial output, README, and this design note.

## Task 1: Red Tests

- [x] **Step 1: Add core success-path tests**

Create `crates/agent-kernel-core/tests/fault_handler.rs` with tests for installing a handler and routing a fault to a handler message.

- [x] **Step 2: Add core failure-path tests**

Create `crates/agent-kernel-core/tests/fault_handler_errors.rs` with tests for missing rollback authority, duplicate handler binding, missing handler route, stale recovered fault route, message-store-full atomicity, and event-log-full atomicity.

- [x] **Step 3: Add facade handler test**

Create `crates/agent-kernel/tests/fault_handler.rs` with one syscall-level test that installs a handler, faults a task, routes the fault, inspects handler records, receives and acknowledges the fault message, recovers, requeues, redispatches, and completes the task.

- [x] **Step 4: Verify red**

Run:

```bash
RUSTC="$(rustup which rustc --toolchain nightly)" RUSTDOC="$(rustup which rustdoc --toolchain nightly)" rustup run nightly cargo test -p agent-kernel-core --test fault_handler
RUSTC="$(rustup which rustc --toolchain nightly)" RUSTDOC="$(rustup which rustdoc --toolchain nightly)" rustup run nightly cargo test -p agent-kernel-core --test fault_handler_errors
RUSTC="$(rustup which rustc --toolchain nightly)" RUSTDOC="$(rustup which rustdoc --toolchain nightly)" rustup run nightly cargo test -p agent-kernel --test fault_handler
```

Expected: fail because `FaultHandlerId`, `FaultHandlerRecord`, handler events, handler errors, message fault payloads, core methods, and facade syscalls do not exist.

## Task 2: Core Handler Store

- [x] **Step 1: Add handler model**

Add `FaultHandlerId` and `FaultHandlerRecord`.

- [x] **Step 2: Extend core storage**

Add trailing `FAULT_HANDLERS` capacity, `fault_handlers`, `fault_handler_len`, and `next_fault_handler` to `KernelCore`.

- [x] **Step 3: Add errors, events, and message payload support**

Add `FaultHandlerStoreFull`, `FaultHandlerAlreadyExists`, and `FaultHandlerNotFound`. Add `FaultHandlerInstalled` and `FaultRouted`. Add `MessageKind::Fault` and `MessagePayload::fault`.

- [x] **Step 4: Refactor mailbox internals for atomic routing**

Add crate-private message capacity and append helpers in `mailbox_store.rs`. Keep `send_message` behavior unchanged while allowing `route_fault_to_handler` to check all capacity before mutation.

- [x] **Step 5: Implement install and route**

Implement `install_fault_handler`, `route_fault_to_handler`, and `fault_handlers()`. Install requires rollback authority and rejects duplicate `(resource, fault_kind)`. Route requires a still-faulted task, rollback authority, a handler binding, message capacity, and two event slots.

- [x] **Step 6: Verify focused core tests**

Run:

```bash
RUSTC="$(rustup which rustc --toolchain nightly)" RUSTDOC="$(rustup which rustdoc --toolchain nightly)" rustup run nightly cargo test -p agent-kernel-core --test fault_handler
RUSTC="$(rustup which rustc --toolchain nightly)" RUSTDOC="$(rustup which rustdoc --toolchain nightly)" rustup run nightly cargo test -p agent-kernel-core --test fault_handler_errors
```

Expected: pass.

## Task 3: Facade, Runtime, And Docs

- [x] **Step 1: Add facade syscalls**

Add `sys_install_fault_handler`, `sys_route_fault_to_handler`, and `fault_handlers()` to `agent-kernel`.

- [x] **Step 2: Update supervisor flow**

Register a handler agent, install the handler for the workspace execution trap, route the trapped task fault to that handler, receive and acknowledge the fault message, recover the faulted task, then requeue and complete it. The output should include `fault_handler_installed` and `fault_routed`.

- [x] **Step 3: Update QEMU event labels**

Add serial labels for `fault_handler_installed` and `fault_routed`.

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

Expected: all commands pass; the no_std scan returns no matches; supervisor output includes `fault_handler_installed`, `task_faulted`, `message_sent`, `fault_routed`, `message_received`, `message_acknowledged`, and `task_fault_recovered` before task completion.

## Self-Review

Spec coverage: the plan covers handler IDs, fixed-capacity records, install authority, duplicate rejection, route authority, still-faulted validation, fault IPC payloads, route events, message/event atomicity, facade syscalls, runtime formatting, QEMU labels, and documentation.

Placeholder scan: no TODO, TBD, or open-ended implementation placeholders remain.

Type consistency: `FaultHandlerId`, `FaultHandlerRecord`, `FaultHandlerStoreFull`, `FaultHandlerAlreadyExists`, `FaultHandlerNotFound`, `FaultHandlerInstalled`, `FaultRouted`, `MessageKind::Fault`, `MessagePayload::fault`, `install_fault_handler`, `route_fault_to_handler`, `sys_install_fault_handler`, and `sys_route_fault_to_handler` are used consistently.
