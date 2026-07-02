# Intent Store V0 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a deterministic intent store so tasks are created from kernel-visible typed intents.

**Architecture:** `agent-kernel-core` gains `IntentId`, `Intent`, `IntentKind`, `VerificationRequirement`, fixed-capacity intent storage, and `IntentDeclared` events. `Task` and task lifecycle events carry an intent id. `agent-kernel` exposes syscall-style intent declaration and read-only intent inspection; supervisor declares an action intent before creating a task.

**Tech Stack:** Rust nightly, no_std core and facade crates, fixed-capacity arrays, Cargo workspace tests, QEMU BIOS boot script.

---

## File Structure

- Create `crates/agent-kernel-core/src/intent.rs` for intent data types.
- Create `crates/agent-kernel-core/src/intent_store.rs` for declaration, lookup, and read-only inspection.
- Create `crates/agent-kernel-core/tests/intent_store.rs` for red tests.
- Create `crates/agent-kernel/tests/intent_lifecycle.rs` for facade tests.
- Modify `crates/agent-kernel-core/src/id.rs`, `event.rs`, `error.rs`, `core.rs`, `task.rs`, `task_store.rs`, `capability_store.rs`, and `lib.rs`.
- Modify all `KernelCore`, `AgentKernel`, and `BootedKernel` generic uses to include `INTENTS`.
- Modify supervisor, boot tests, x86 serial labels, README, and QEMU expectations only where event sequences change.

## Task 1: Core Intent Red Tests

**Files:**
- Create: `crates/agent-kernel-core/tests/intent_store.rs`

- [ ] **Step 1: Add intent tests**

Create `crates/agent-kernel-core/tests/intent_store.rs` with tests for:

```rust
use agent_kernel_core::{
    AgentId, EventKind, IntentId, IntentKind, KernelCore, KernelError, Operation,
    OperationSet, ResourceKind, TaskId, VerificationRequirement,
};

type TestCore = KernelCore<4, 8, 32, 4, 6, 4>;

#[test]
fn declare_intent_records_typed_intent() {
    let mut core = TestCore::new();
    let agent = AgentId::new(1);
    let resource = core.register_resource(ResourceKind::Workspace, None).unwrap();
    let capability = core
        .grant_capability(agent, resource, OperationSet::only(Operation::Act))
        .unwrap();

    let intent = core
        .declare_intent(
            agent,
            capability,
            resource,
            IntentKind::Act,
            VerificationRequirement::Required,
        )
        .unwrap();

    assert_eq!(intent, IntentId::new(1));
    assert_eq!(core.intents().len(), 1);
    assert_eq!(core.intents()[0].id, intent);
    assert_eq!(core.intents()[0].owner, agent);
    assert_eq!(core.intents()[0].resource, resource);
    assert_eq!(core.intents()[0].kind, IntentKind::Act);
    assert_eq!(
        core.intents()[0].verification,
        VerificationRequirement::Required
    );
    assert_eq!(core.events()[1].kind, EventKind::IntentDeclared);
    assert_eq!(core.events()[1].intent, Some(intent));
    assert_eq!(core.events()[1].intent_kind, Some(IntentKind::Act));
    assert_eq!(
        core.events()[1].verification,
        VerificationRequirement::Required
    );
}
```

Also include tests for:

```rust
#[test]
fn declare_intent_requires_matching_operation_capability() { /* Observe cap cannot declare Act intent */ }

#[test]
fn declare_intent_returns_intent_store_full_without_mutation() { /* KernelCore::<1, 1, 4, 0, 0, 0> */ }

#[test]
fn declare_intent_returns_event_log_full_without_mutation() { /* event slot consumed by grant */ }

#[test]
fn create_task_from_intent_binds_task_and_event_to_intent() { /* Task.intent and TaskCreated.intent */ }

#[test]
fn create_task_rejects_other_agents_intent_without_mutation() { /* IntentAgentMismatch */ }

#[test]
fn task_lifecycle_events_carry_task_intent() { /* accepted, queued, dispatched, completed, verified */ }
```

- [ ] **Step 2: Run red core test**

Run:

```bash
PATH="$HOME/.cargo/bin:$PATH" RUSTC="$(rustup which rustc --toolchain nightly)" rustup run nightly cargo test -p agent-kernel-core --test intent_store
```

Expected: compile failures for missing `IntentId`, `IntentKind`, `VerificationRequirement`, `declare_intent`, `intents`, `EventKind::IntentDeclared`, `Event::intent`, and `Task::intent`.

## Task 2: Core Intent Model

**Files:**
- Create: `crates/agent-kernel-core/src/intent.rs`
- Create: `crates/agent-kernel-core/src/intent_store.rs`
- Modify: `crates/agent-kernel-core/src/id.rs`
- Modify: `crates/agent-kernel-core/src/error.rs`
- Modify: `crates/agent-kernel-core/src/event.rs`
- Modify: `crates/agent-kernel-core/src/core.rs`
- Modify: `crates/agent-kernel-core/src/lib.rs`

- [ ] **Step 1: Add `IntentId`**

Add `IntentId` to `id.rs` using the same wrapper pattern as `TaskId`.

- [ ] **Step 2: Add intent types**

Create `intent.rs` with:

```rust
use crate::{AgentId, IntentId, Operation, ResourceId};

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum IntentKind {
    Observe,
    Act,
    Verify,
    Checkpoint,
    Rollback,
}

impl IntentKind {
    pub const fn required_operation(self) -> Operation {
        match self {
            Self::Observe => Operation::Observe,
            Self::Act => Operation::Act,
            Self::Verify => Operation::Verify,
            Self::Checkpoint => Operation::Checkpoint,
            Self::Rollback => Operation::Rollback,
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum VerificationRequirement {
    Optional,
    Required,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Intent {
    pub id: IntentId,
    pub owner: AgentId,
    pub resource: ResourceId,
    pub kind: IntentKind,
    pub verification: VerificationRequirement,
}

impl Intent {
    pub(crate) const fn empty() -> Self { /* zero id, zero agent/resource, Act, Optional */ }
}
```

- [ ] **Step 3: Extend errors and events**

Add `IntentStoreFull`, `IntentNotFound`, and `IntentAgentMismatch` to `KernelError`.

Add `IntentDeclared` to `EventKind`.

Add fields to `Event`:

```rust
pub intent: Option<IntentId>,
pub intent_kind: Option<IntentKind>,
pub verification: VerificationRequirement,
```

Set empty defaults to `None`, `None`, and `VerificationRequirement::Optional`.

- [ ] **Step 4: Extend `KernelCore` generics and state**

Change `KernelCore` to:

```rust
pub struct KernelCore<
    const RESOURCES: usize,
    const CAPS: usize,
    const EVENTS: usize,
    const INTENTS: usize,
    const TASKS: usize,
    const RUN_QUEUE: usize,
> {
    pub(crate) intents: [Intent; INTENTS],
    pub(crate) intent_len: usize,
    pub(crate) next_intent: u64,
    // existing fields remain
}
```

Update every core module impl generic list to include `INTENTS`.

- [ ] **Step 5: Add intent store**

Create `intent_store.rs` with:

```rust
pub fn declare_intent(...) -> Result<IntentId, KernelError> { /* authorize required operation, capacity, record IntentDeclared */ }
pub fn intents(&self) -> &[Intent] { &self.intents[..self.intent_len] }
pub(crate) fn find_intent(&self, id: IntentId) -> Result<Intent, KernelError> { ... }
```

`IntentDeclared` records `operation: Some(kind.required_operation())`,
`intent: Some(id)`, `intent_kind: Some(kind)`, and the verification requirement.

- [ ] **Step 6: Export modules**

Add `mod intent; mod intent_store;` and export `Intent`, `IntentKind`, and `VerificationRequirement`.

- [ ] **Step 7: Run focused red-green check**

Run the focused intent test. Expected: remaining compile failures now point to task binding and generic users not yet updated.

## Task 3: Task Binding And Generic Updates

**Files:**
- Modify: `crates/agent-kernel-core/src/task.rs`
- Modify: `crates/agent-kernel-core/src/task_store.rs`
- Modify: `crates/agent-kernel-core/src/capability_store.rs`
- Modify: `crates/agent-kernel-core/src/core.rs`
- Modify: `crates/agent-kernel-core/src/scheduler.rs`
- Modify: all core tests using `KernelCore<...>`

- [ ] **Step 1: Extend `Task`**

Add `pub intent: IntentId` and set `IntentId::new(0)` in `Task::empty()`.

- [ ] **Step 2: Change task creation**

Change:

```rust
pub fn create_task(&mut self, agent, capability, resource) -> Result<TaskId, KernelError>
```

to:

```rust
pub fn create_task(&mut self, agent, capability, intent) -> Result<TaskId, KernelError>
```

It finds the intent, requires owner match, authorizes `intent.kind.required_operation()` on `intent.resource`, stores `Task.intent`, and records `TaskCreated` with `intent: Some(intent)`.

- [ ] **Step 3: Include intent in task events**

Update `record_task_event` to set `intent: Some(task_record.intent)`.

Update `derive_task_capability` lifecycle event to include the task intent by looking up the task before recording `CapabilityDerived`.

- [ ] **Step 4: Update non-intent event literals**

All non-intent events in `core.rs`, `scheduler.rs`, and capability lifecycle events get `intent: None` unless they are task-scoped derived capability events.

- [ ] **Step 5: Update core tests**

Update each core test to add the new `INTENTS` const and declare an intent before task creation. Focus helpers should create an `IntentKind::Act` intent with `VerificationRequirement::Required`.

- [ ] **Step 6: Run core tests**

Run:

```bash
PATH="$HOME/.cargo/bin:$PATH" RUSTC="$(rustup which rustc --toolchain nightly)" rustup run nightly cargo test -p agent-kernel-core
```

Expected: all core tests pass.

## Task 4: Facade, Supervisor, Boot, And Docs

**Files:**
- Modify: `crates/agent-kernel/src/lib.rs`
- Modify: `crates/agent-kernel/src/scheduler.rs`
- Modify: all `agent-kernel` tests
- Modify: `crates/agent-kernel-boot/src/lib.rs`
- Modify: `crates/agent-kernel-boot/tests/boot_flow.rs`
- Modify: `crates/agent-kernel-x86_64/src/main.rs`
- Modify: `crates/agent-supervisor/src/main.rs`
- Modify: `crates/agent-supervisor/tests/supervisor_flow.rs`
- Modify: `README.md`

- [ ] **Step 1: Update facade generics and APIs**

Add `INTENTS` to `AgentKernel` generics.

Add:

```rust
pub fn sys_declare_intent(...)
pub fn intents(&self) -> &[Intent]
```

Change `sys_create_task` to accept `IntentId`.

- [ ] **Step 2: Update boot generics**

Add `INTENTS` to `BootedKernel` generics. Boot can instantiate with intent capacity `0` because it does not declare intents.

- [ ] **Step 3: Update supervisor flow**

After rollback, declare:

```rust
let intent = kernel.sys_declare_intent(
    agent,
    owner_capability,
    workspace,
    IntentKind::Act,
    VerificationRequirement::Required,
)?;
let task = kernel.sys_create_task(agent, owner_capability, intent)?;
```

Add formatter output for `IntentDeclared`:

```text
event[n] intent_declared agent=1 resource=1 intent=1
```

- [ ] **Step 4: Update exhaustive event matches**

Add `EventKind::IntentDeclared` to supervisor and x86 serial output.

- [ ] **Step 5: Update README**

Document that the supervisor declares an action intent before task creation and update expected supervisor output to include `intent_declared`.

- [ ] **Step 6: Run workspace tests**

Run:

```bash
PATH="$HOME/.cargo/bin:$PATH" RUSTC="$(rustup which rustc --toolchain nightly)" rustup run nightly cargo test --workspace
```

Expected: all tests pass.

## Task 5: Verification And Publish

**Files:**
- All implementation and test files changed above.

- [ ] **Step 1: Run formatting**

Run:

```bash
PATH="$HOME/.cargo/bin:$PATH" rustup run nightly cargo fmt --check
```

- [ ] **Step 2: Run workspace tests**

Run:

```bash
PATH="$HOME/.cargo/bin:$PATH" RUSTC="$(rustup which rustc --toolchain nightly)" rustup run nightly cargo test --workspace
```

- [ ] **Step 3: Run supervisor**

Run:

```bash
PATH="$HOME/.cargo/bin:$PATH" RUSTC="$(rustup which rustc --toolchain nightly)" rustup run nightly cargo run -p agent-supervisor
```

Expected: output includes `intent_declared` before `task_created`.

- [ ] **Step 4: Run QEMU**

Run:

```bash
scripts/run-qemu.sh
```

Expected: unchanged boot trace still passes.

- [ ] **Step 5: Commit and push**

Run:

```bash
git status --short
git add README.md crates docs scripts
git commit -m "feat: add intent store"
git push
```

Expected: `origin/main` receives the intent store implementation.

## Self-Review

Spec coverage:

- Intent declaration, event recording, task binding, and facade/supervisor integration are covered.
- Natural-language payloads, policy engines, and intent mutation are explicitly deferred.

Placeholder scan:

- This plan contains concrete file paths, tests, commands, and expected outputs.

Type consistency:

- `IntentId`, `IntentKind`, `VerificationRequirement`, `IntentDeclared`, `intent`, and `intent_kind` match the design spec.
