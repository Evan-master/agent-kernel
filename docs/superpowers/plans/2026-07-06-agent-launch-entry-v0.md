# Agent Launch Entry V0 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a native launch entry transition that binds an active agent to a resource, capability, and optional intent before runtime work proceeds.

**Architecture:** `agent-kernel-core` owns the no_std launch entry model, authority checks, fixed-capacity storage, and `AgentLaunched` event. `agent-kernel` exposes the syscall wrapper and read-only inspection. Boot handoff and the supervisor use launch to show the runtime admission boundary without adding host processes, POSIX compatibility, or binary loading.

**Tech Stack:** Rust nightly, no_std core and facade crates, fixed-capacity arrays, Cargo workspace tests, QEMU BIOS boot script.

---

## File Structure

- Create `crates/agent-kernel-core/src/agent_entry.rs` for `AgentEntryKind` and `AgentEntryRecord`.
- Create `crates/agent-kernel-core/src/agent_launch.rs` for launch validation, entry inspection, and event recording.
- Modify `crates/agent-kernel-core/src/core.rs` to store `[AgentEntryRecord; AGENTS]`.
- Modify `crates/agent-kernel-core/src/event.rs` to add `AgentLaunched`.
- Modify `crates/agent-kernel-core/src/error.rs` to add launch-specific errors.
- Modify `crates/agent-kernel-core/src/lib.rs` to export entry types and load the launch modules.
- Modify `crates/agent-kernel/src/agent.rs` to expose `sys_launch_agent`, `agent_entries`, and `agent_entry`.
- Add `crates/agent-kernel-core/tests/agent_launch.rs`.
- Add `crates/agent-kernel-core/tests/agent_launch_errors.rs`.
- Add `crates/agent-kernel/tests/agent_launch.rs`.
- Modify boot handoff, QEMU event labels, supervisor flow/tests, README, and this plan.

## Task 1: Red Tests

- [x] **Step 1: Add core success tests**

Create `crates/agent-kernel-core/tests/agent_launch.rs` with tests proving launch records an entry, records `AgentLaunched`, stores optional declared action intent references, and exposes `agent_entry(agent)`.

- [x] **Step 2: Add core failure tests**

Create `crates/agent-kernel-core/tests/agent_launch_errors.rs` with tests for unknown agent, missing act authority, duplicate launch, mismatched intent owner, mismatched intent resource, non-action intent kind, non-declared intent status, and event-log-full atomicity.

- [x] **Step 3: Add facade test**

Create `crates/agent-kernel/tests/agent_launch.rs` proving `AgentKernel::sys_launch_agent`, `agent_entries()`, and `agent_entry(agent)` expose the same behavior.

- [x] **Step 4: Verify red**

Run:

```bash
RUSTC="$(rustup which rustc --toolchain nightly)" RUSTDOC="$(rustup which rustdoc --toolchain nightly)" rustup run nightly cargo test -p agent-kernel-core --test agent_launch --test agent_launch_errors -p agent-kernel --test agent_launch
```

Expected: fail because `AgentEntryKind`, `AgentEntryRecord`, `AgentLaunched`, `launch_agent`, `sys_launch_agent`, and launch errors do not exist.

## Task 2: Core Launch Model

- [x] **Step 1: Add entry types**

Add:

```rust
pub enum AgentEntryKind {
    Bootstrap,
    Supervisor,
    Worker,
}

pub struct AgentEntryRecord {
    pub agent: AgentId,
    pub resource: ResourceId,
    pub capability: CapabilityId,
    pub kind: AgentEntryKind,
    pub intent: Option<IntentId>,
}
```

- [x] **Step 2: Add fixed-capacity entry storage**

Add `[AgentEntryRecord; AGENTS]` and `agent_entry_len` to `KernelCore`, initialize with `AgentEntryRecord::empty()`, and expose `agent_entries()` plus `agent_entry(agent)`.

- [x] **Step 3: Implement launch validation and event recording**

Implement `launch_agent` with active-agent validation, duplicate-launch rejection, `Operation::Act` authorization, optional declared action intent validation, entry capacity, event capacity, entry insertion, and `AgentLaunched` recording.

- [x] **Step 4: Verify focused core tests**

Run:

```bash
RUSTC="$(rustup which rustc --toolchain nightly)" RUSTDOC="$(rustup which rustdoc --toolchain nightly)" rustup run nightly cargo test -p agent-kernel-core --test agent_launch --test agent_launch_errors
```

Expected: pass.

## Task 3: Facade, Runtime, And Docs

- [x] **Step 1: Add facade syscall and inspection**

Expose `AgentEntryKind`, `AgentEntryRecord`, `sys_launch_agent`, `agent_entries()`, and `agent_entry(agent)` through `agent-kernel`.

- [x] **Step 2: Update boot handoff**

Launch the bootstrap agent immediately after its observe/act/verify capability grant. Expected QEMU boot labels:

```text
event[1] agent_registered
event[2] capability_granted
event[3] agent_launched
event[4] observation
event[5] action
event[6] verification
```

- [x] **Step 3: Update supervisor flow and formatting**

Launch the owner agent after the initial workspace capability grant and format it as:

```text
event[5] agent_launched agent=1 resource=1 capability=1
```

Shift later expected supervisor event numbers by one.

- [x] **Step 4: Update README**

Document agent launch entries in current scope, current behavior, first-class events, boot handoff, QEMU output, and supervisor output.

- [x] **Step 5: Final verification**

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

Expected: all commands pass; no_std scan returns no matches; boot and supervisor output include `agent_launched`.

## Self-Review

Spec coverage: this plan covers entry metadata, launch authority, optional intent validation, atomicity, event recording, facade exposure, runtime output, QEMU label, supervisor demo, and documentation.

Plan scan: no open-ended implementation notes remain.

Type consistency: `AgentEntryKind`, `AgentEntryRecord`, `launch_agent`, `sys_launch_agent`, `AgentLaunched`, `AgentAlreadyLaunched`, and `AgentEntryStoreFull` are used consistently.
