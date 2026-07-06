# Agent Mailbox IPC V0 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add deterministic, fixed-capacity kernel mailboxes so active agents can exchange typed messages through native Agent Kernel IPC.

**Architecture:** `agent-kernel-core` owns message records, FIFO receive semantics, active-agent checks, and replayable events. `agent-kernel` exposes syscall-style wrappers. Supervisor and QEMU formatters learn the new event kinds without moving reasoning or host I/O into kernel space.

**Tech Stack:** Rust nightly, no_std core and facade crates, fixed-capacity arrays, Cargo workspace tests, QEMU BIOS boot script.

---

## File Structure

- Modify `crates/agent-kernel-core/src/id.rs` for `MessageId`.
- Create `crates/agent-kernel-core/src/message.rs` for message payloads, records, kinds, and statuses.
- Create `crates/agent-kernel-core/src/mailbox_store.rs` for deterministic send, receive, acknowledgement, and lookup.
- Modify `crates/agent-kernel-core/src/core.rs`, `event.rs`, `error.rs`, and `lib.rs`.
- Modify every `KernelCore<...>` impl header in core source to include the trailing `MESSAGES` capacity.
- Modify `crates/agent-kernel/src/lib.rs` and `scheduler.rs` for `AgentKernel<..., MESSAGES>`.
- Create `crates/agent-kernel/src/mailbox.rs` for mailbox syscall wrappers and message inspection.
- Add `crates/agent-kernel-core/tests/mailbox_ipc.rs`.
- Add `crates/agent-kernel-core/tests/mailbox_ipc_errors.rs`.
- Add `crates/agent-kernel/tests/mailbox_ipc.rs`.
- Create `crates/agent-supervisor/src/format.rs` for deterministic event formatting.
- Modify `crates/agent-supervisor/src/main.rs`, supervisor tests, QEMU serial output, README, and this design note.

## Task 1: Red Tests

- [x] **Step 1: Add core mailbox tests**

Create `crates/agent-kernel-core/tests/mailbox_ipc.rs` with send, FIFO receive, and acknowledge success-path tests. Create `crates/agent-kernel-core/tests/mailbox_ipc_errors.rs` with empty mailbox, active-agent enforcement, wrong-recipient acknowledgement, status mismatch, store-full atomicity, and event-log-full atomicity tests.

- [x] **Step 2: Add facade mailbox test**

Create `crates/agent-kernel/tests/mailbox_ipc.rs` with one syscall-level test that sends, receives, acknowledges, and inspects `messages()`.

- [x] **Step 3: Verify red**

Run:

```bash
RUSTC="$(rustup which rustc --toolchain nightly)" RUSTDOC="$(rustup which rustdoc --toolchain nightly)" rustup run nightly cargo test -p agent-kernel-core --test mailbox_ipc
RUSTC="$(rustup which rustc --toolchain nightly)" RUSTDOC="$(rustup which rustdoc --toolchain nightly)" rustup run nightly cargo test -p agent-kernel-core --test mailbox_ipc_errors
RUSTC="$(rustup which rustc --toolchain nightly)" RUSTDOC="$(rustup which rustdoc --toolchain nightly)" rustup run nightly cargo test -p agent-kernel --test mailbox_ipc
```

Expected: fail because message types, errors, events, stores, and syscalls do not exist.

## Task 2: Core Mailbox Store

- [x] **Step 1: Add message model**

Add `MessageId`, `MessageKind`, `MessageStatus`, `MessagePayload`, and `MessageRecord`.

- [x] **Step 2: Extend core storage**

Add trailing `MESSAGES` capacity, `messages`, `message_len`, and `next_message` to `KernelCore`.

- [x] **Step 3: Add errors and events**

Add `MessageStoreFull`, `MessageNotFound`, `MessageAgentMismatch`, `MessageStatusMismatch`, `MailboxEmpty`, plus `MessageSent`, `MessageReceived`, and `MessageAcknowledged`.

- [x] **Step 4: Implement send, receive, ack, and inspection**

Implement `send_message`, `receive_message`, `acknowledge_message`, `messages`, and internal message lookup helpers.

- [x] **Step 5: Verify focused core tests**

Run:

```bash
RUSTC="$(rustup which rustc --toolchain nightly)" RUSTDOC="$(rustup which rustdoc --toolchain nightly)" rustup run nightly cargo test -p agent-kernel-core --test mailbox_ipc
RUSTC="$(rustup which rustc --toolchain nightly)" RUSTDOC="$(rustup which rustdoc --toolchain nightly)" rustup run nightly cargo test -p agent-kernel-core --test mailbox_ipc_errors
```

Expected: pass.

## Task 3: Facade, Runtime, And Docs

- [x] **Step 1: Add facade syscalls**

Add `sys_send_message`, `sys_receive_message`, `sys_acknowledge_message`, and `messages()`.

- [x] **Step 2: Update supervisor flow**

After the existing task verification flow, send one notify message from the owner agent to the target agent, receive it, acknowledge it, and print the resulting events.

- [x] **Step 3: Update QEMU event labels**

Add serial labels for `message_sent`, `message_received`, and `message_acknowledged`.

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

Expected: all commands pass and the supervisor output includes one message send, receive, and acknowledgement after the task verification flow.

## Self-Review

Spec coverage: the plan covers fixed-capacity storage, active-agent authority, FIFO receive, acknowledgement, atomic failure paths, facade syscalls, runtime formatting, and documentation.

Placeholder scan: no TODO, TBD, or open-ended implementation placeholders remain.

Type consistency: `MessageId`, `MessageRecord`, `MessagePayload`, `MessageKind`, `MessageStatus`, `MessageStoreFull`, `MessageNotFound`, `MessageAgentMismatch`, `MessageStatusMismatch`, `MailboxEmpty`, `MessageSent`, `MessageReceived`, `MessageAcknowledged`, `send_message`, `receive_message`, `acknowledge_message`, `sys_send_message`, `sys_receive_message`, `sys_acknowledge_message`, and `messages()` are used consistently.
