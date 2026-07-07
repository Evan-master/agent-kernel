# Agent Driver Binding V0 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add deterministic agent driver bindings and fixed-width device event lifecycle records so Agent Kernel can name which agent controls a device-like resource.

**Architecture:** `agent-kernel-core` owns the no_std driver binding and device event records, fixed-capacity stores, authorization checks, and replayable events. `agent-kernel` exposes syscall-style wrappers. `agent-supervisor` simulates a device event source and formats the new event labels without moving host I/O into kernel crates.

**Tech Stack:** Rust 2021, no_std-compatible core/facade crates, fixed-capacity arrays, existing Cargo workspace and nightly toolchain used by QEMU/bootloader paths.

---

### Task 1: Core Driver Binding Model

**Files:**
- Create: `crates/agent-kernel-core/src/driver.rs`
- Create: `crates/agent-kernel-core/src/driver_event.rs`
- Modify: `crates/agent-kernel-core/src/id.rs`
- Modify: `crates/agent-kernel-core/src/event.rs`
- Modify: `crates/agent-kernel-core/src/error.rs`
- Modify: `crates/agent-kernel-core/src/core.rs`
- Modify: `crates/agent-kernel-core/src/lib.rs`
- Test: `crates/agent-kernel-core/tests/driver_binding.rs`

- [ ] **Step 1: Write the failing success-path binding test**

```rust
use agent_kernel_core::{
    AgentId, DriverBindingId, EventKind, KernelCore, Operation, OperationSet, ResourceKind,
};

type TestKernel = KernelCore<4, 4, 4, 8, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 2, 0>;

#[test]
fn bind_driver_records_binding_and_event() {
    let mut core = TestKernel::new();
    let owner = AgentId::new(1);
    let driver = AgentId::new(2);

    core.register_agent(owner).unwrap();
    core.register_agent(driver).unwrap();
    let device = core.register_resource(ResourceKind::Device, None).unwrap();
    let capability = core
        .grant_capability(
            owner,
            device,
            OperationSet::only(Operation::Delegate),
        )
        .unwrap();

    let binding = core
        .bind_driver(owner, capability, device, driver)
        .expect("driver should bind");

    assert_eq!(binding, DriverBindingId::new(1));
    assert_eq!(core.driver_bindings().len(), 1);
    assert_eq!(core.driver_bindings()[0].driver, driver);
    assert_eq!(core.driver_bindings()[0].resource, device);
    let event = core.events().last().unwrap();
    assert_eq!(event.kind, EventKind::DriverBound);
    assert_eq!(event.driver_binding, Some(binding));
    assert_eq!(event.target_agent, Some(driver));
}
```

- [ ] **Step 2: Run test to verify it fails**

Run:

```bash
RUSTC="$(rustup which --toolchain nightly rustc)" RUSTDOC="$(rustup which --toolchain nightly rustdoc)" rustup run nightly cargo test -p agent-kernel-core --test driver_binding bind_driver_records_binding_and_event
```

Expected: compile failure because `DriverBindingId`, `bind_driver`, `driver_bindings`, and `Event::driver_binding` do not exist.

- [ ] **Step 3: Add ID, record, event fields, and KernelCore storage**

Implement:

```rust
pub struct DriverBindingId(u64);

pub struct DriverBindingRecord {
    pub id: DriverBindingId,
    pub installer: AgentId,
    pub resource: ResourceId,
    pub resource_kind: ResourceKind,
    pub driver: AgentId,
}
```

Add `DriverBound` to `EventKind`, `driver_binding: Option<DriverBindingId>` to `Event`, `DriverBindingStoreFull`, `DriverBindingNotFound`, and `DriverBindingAlreadyExists` to `KernelError`, and trailing `DRIVER_BINDINGS` storage in `KernelCore`.

- [ ] **Step 4: Implement minimal `bind_driver`**

Implement `bind_driver` in `driver.rs` with active-agent checks, active-resource lookup, device-like kind validation, `Operation::Delegate` authorization, duplicate binding rejection, capacity check, event-slot check, storage append, and `DriverBound` event recording.

- [ ] **Step 5: Run focused test to verify green**

Run the same focused command. Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add crates/agent-kernel-core/src crates/agent-kernel-core/tests/driver_binding.rs
git commit -m "feat: bind agents as resource drivers"
```

### Task 2: Driver Binding Failure Atomicity

**Files:**
- Modify: `crates/agent-kernel-core/src/driver.rs`
- Test: `crates/agent-kernel-core/tests/driver_binding.rs`

- [ ] **Step 1: Add failing binding error tests**

Add tests named:

```rust
bind_driver_requires_delegate_authority_without_mutation
bind_driver_rejects_inactive_driver_without_allocation
bind_driver_rejects_non_device_resource_without_mutation
bind_driver_rejects_duplicate_without_second_event
bind_driver_store_full_leaves_event_log_unchanged
bind_driver_event_log_full_leaves_no_binding
```

Each test must assert both returned `KernelError` and unchanged `driver_bindings().len()` or unchanged `events().len()` as applicable.

- [ ] **Step 2: Run tests to verify they fail**

Run:

```bash
RUSTC="$(rustup which --toolchain nightly rustc)" RUSTDOC="$(rustup which --toolchain nightly rustdoc)" rustup run nightly cargo test -p agent-kernel-core --test driver_binding
```

Expected: failures for missing or incorrect error/atomic paths.

- [ ] **Step 3: Fix binding validation order**

Ensure validation order is:

1. installer active,
2. driver active,
3. active resource lookup,
4. device-like kind check,
5. delegate authorization,
6. duplicate binding lookup,
7. binding capacity,
8. event capacity,
9. mutate.

- [ ] **Step 4: Run tests to verify green**

Run the same command. Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/agent-kernel-core/src/driver.rs crates/agent-kernel-core/tests/driver_binding.rs
git commit -m "test: cover driver binding failures"
```

### Task 3: Device Event Lifecycle

**Files:**
- Create: `crates/agent-kernel-core/src/device_event.rs`
- Modify: `crates/agent-kernel-core/src/driver_event.rs`
- Modify: `crates/agent-kernel-core/src/event.rs`
- Modify: `crates/agent-kernel-core/src/error.rs`
- Modify: `crates/agent-kernel-core/src/core.rs`
- Modify: `crates/agent-kernel-core/src/lib.rs`
- Test: `crates/agent-kernel-core/tests/device_event.rs`

- [ ] **Step 1: Write failing lifecycle test**

```rust
use agent_kernel_core::{
    AgentId, DeviceEventId, DeviceEventKind, DeviceEventPayload, DeviceEventStatus, EventKind,
    KernelCore, Operation, OperationSet, ResourceKind,
};

type TestKernel = KernelCore<4, 4, 6, 12, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 2, 2>;

#[test]
fn device_event_reaches_acknowledged_through_bound_driver() {
    let mut core = TestKernel::new();
    let owner = AgentId::new(1);
    let driver = AgentId::new(2);
    core.register_agent(owner).unwrap();
    core.register_agent(driver).unwrap();
    let device = core.register_resource(ResourceKind::Device, None).unwrap();
    let owner_capability = core
        .grant_capability(
            owner,
            device,
            OperationSet::empty()
                .with(Operation::Delegate)
                .with(Operation::Act),
        )
        .unwrap();
    let driver_capability = core
        .grant_capability(
            driver,
            device,
            OperationSet::empty()
                .with(Operation::Observe)
                .with(Operation::Act),
        )
        .unwrap();
    let binding = core.bind_driver(owner, owner_capability, device, driver).unwrap();

    let event = core
        .raise_device_event(
            owner,
            owner_capability,
            device,
            DeviceEventKind::StateChanged,
            DeviceEventPayload { code: 7, value: 9 },
        )
        .unwrap();
    assert_eq!(event, DeviceEventId::new(1));
    assert_eq!(core.device_events()[0].binding, binding);
    assert_eq!(core.device_events()[0].status, DeviceEventStatus::Raised);

    core.deliver_device_event(driver, driver_capability, event).unwrap();
    assert_eq!(core.device_events()[0].status, DeviceEventStatus::Delivered);
    core.acknowledge_device_event(driver, driver_capability, event).unwrap();
    assert_eq!(core.device_events()[0].status, DeviceEventStatus::Acknowledged);

    let kinds: [EventKind; 4] = [
        core.events()[2].kind,
        core.events()[3].kind,
        core.events()[4].kind,
        core.events()[5].kind,
    ];
    assert_eq!(
        kinds,
        [
            EventKind::DriverBound,
            EventKind::DeviceEventRaised,
            EventKind::DeviceEventDelivered,
            EventKind::DeviceEventAcknowledged,
        ]
    );
}
```

- [ ] **Step 2: Run lifecycle test to verify it fails**

Run:

```bash
RUSTC="$(rustup which --toolchain nightly rustc)" RUSTDOC="$(rustup which --toolchain nightly rustdoc)" rustup run nightly cargo test -p agent-kernel-core --test device_event device_event_reaches_acknowledged_through_bound_driver
```

Expected: compile failure for missing device event types and methods.

- [ ] **Step 3: Implement device event records and store**

Implement `DeviceEventKind`, `DeviceEventPayload`, `DeviceEventStatus`, `DeviceEventRecord`, `DeviceEventId`, event fields, event kinds, `DeviceEventStoreFull`, `DeviceEventNotFound`, `DeviceEventStatusMismatch`, and trailing `DEVICE_EVENTS` storage.

- [ ] **Step 4: Implement lifecycle methods**

Implement `raise_device_event`, `deliver_device_event`, `acknowledge_device_event`, `device_events`, and internal lookup helpers. Validate before mutation exactly as the spec describes.

- [ ] **Step 5: Add and run error tests**

Add tests named:

```rust
raise_device_event_requires_act_authority_without_mutation
raise_device_event_requires_existing_binding_without_mutation
raise_device_event_store_full_leaves_event_log_unchanged
raise_device_event_log_full_leaves_no_device_event
deliver_device_event_requires_bound_driver_without_mutation
deliver_device_event_requires_observe_authority_without_mutation
acknowledge_device_event_requires_act_authority_without_mutation
repeated_delivery_or_acknowledgement_is_rejected_without_mutation
retired_device_resource_rejects_event_transitions
```

Run:

```bash
RUSTC="$(rustup which --toolchain nightly rustc)" RUSTDOC="$(rustup which --toolchain nightly rustdoc)" rustup run nightly cargo test -p agent-kernel-core --test device_event
```

Expected after implementation: PASS.

- [ ] **Step 6: Commit**

```bash
git add crates/agent-kernel-core/src crates/agent-kernel-core/tests/device_event.rs
git commit -m "feat: add device event lifecycle"
```

### Task 4: Kernel Facade Driver Syscalls

**Files:**
- Create: `crates/agent-kernel/src/driver.rs`
- Modify: `crates/agent-kernel/src/lib.rs`
- Test: `crates/agent-kernel/tests/driver.rs`

- [ ] **Step 1: Write failing facade lifecycle test**

```rust
use agent_kernel_core::{
    AgentId, DeviceEventKind, DeviceEventPayload, DeviceEventStatus, EventKind, Operation,
    OperationSet, ResourceKind,
};
use agent_kernel::AgentKernel;

type TestKernel = AgentKernel<4, 4, 6, 12, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 2, 2>;

#[test]
fn driver_syscalls_expose_device_event_lifecycle() {
    let mut kernel = TestKernel::new();
    let owner = AgentId::new(1);
    let driver = AgentId::new(2);
    kernel.sys_register_agent(owner).unwrap();
    kernel.sys_register_agent(driver).unwrap();
    let device = kernel.sys_register_resource(ResourceKind::Device, None).unwrap();
    let owner_capability = kernel
        .sys_grant(
            owner,
            device,
            OperationSet::empty()
                .with(Operation::Delegate)
                .with(Operation::Act),
        )
        .unwrap();
    let driver_capability = kernel
        .sys_grant(
            driver,
            device,
            OperationSet::empty()
                .with(Operation::Observe)
                .with(Operation::Act),
        )
        .unwrap();
    let binding = kernel
        .sys_bind_driver(owner, owner_capability, device, driver)
        .unwrap();
    let event = kernel
        .sys_raise_device_event(
            owner,
            owner_capability,
            device,
            DeviceEventKind::StateChanged,
            DeviceEventPayload { code: 1, value: 2 },
        )
        .unwrap();
    kernel
        .sys_deliver_device_event(driver, driver_capability, event)
        .unwrap();
    kernel
        .sys_acknowledge_device_event(driver, driver_capability, event)
        .unwrap();

    assert_eq!(kernel.driver_bindings()[0].id, binding);
    assert_eq!(kernel.device_events()[0].status, DeviceEventStatus::Acknowledged);
    assert_eq!(kernel.events().last().unwrap().kind, EventKind::DeviceEventAcknowledged);
}
```

- [ ] **Step 2: Run facade test to verify it fails**

Run:

```bash
RUSTC="$(rustup which --toolchain nightly rustc)" RUSTDOC="$(rustup which --toolchain nightly rustdoc)" rustup run nightly cargo test -p agent-kernel --test driver
```

Expected: compile failure for missing facade module, generics, syscalls, and inspectors.

- [ ] **Step 3: Implement facade wrappers**

Add `mod driver;`, extend `AgentKernel` trailing const generics with `DRIVER_BINDINGS` and `DEVICE_EVENTS`, pass them through to `KernelCore`, and expose:

```rust
sys_bind_driver
sys_raise_device_event
sys_deliver_device_event
sys_acknowledge_device_event
driver_bindings
device_events
```

- [ ] **Step 4: Run facade tests to verify green**

Run the same command. Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/agent-kernel/src crates/agent-kernel/tests/driver.rs
git commit -m "feat: expose driver syscalls"
```

### Task 5: Supervisor Flow, Formatting, And Docs

**Files:**
- Modify: `crates/agent-supervisor/src/main.rs`
- Modify: `crates/agent-supervisor/src/format.rs`
- Modify: `crates/agent-supervisor/tests/supervisor_flow.rs`
- Modify: `crates/agent-kernel-x86_64/src/main.rs`
- Modify: `README.md`

- [ ] **Step 1: Add failing supervisor assertions**

In `crates/agent-supervisor/tests/supervisor_flow.rs`, assert output contains, in order:

```text
driver_bound
device_event_raised
device_event_delivered
device_event_acknowledged
```

- [ ] **Step 2: Run supervisor test to verify it fails**

Run:

```bash
RUSTC="$(rustup which --toolchain nightly rustc)" RUSTDOC="$(rustup which --toolchain nightly rustdoc)" rustup run nightly cargo test -p agent-supervisor --test supervisor_flow
```

Expected: failure because the supervisor flow and formatter do not emit driver labels.

- [ ] **Step 3: Implement formatting**

Add match arms for the four new event kinds. Format:

```text
event[N] driver_bound agent=A resource=R capability=C driver_binding=B target_agent=D
event[N] device_event_raised agent=A resource=R capability=C driver_binding=B device_event=E kind=K code=X value=Y
event[N] device_event_delivered agent=A resource=R capability=C driver_binding=B device_event=E kind=K code=X value=Y
event[N] device_event_acknowledged agent=A resource=R capability=C driver_binding=B device_event=E kind=K code=X value=Y
```

Update the x86 serial label match so the enum remains exhaustively handled even though boot does not raise device events.

- [ ] **Step 4: Implement supervisor demonstration**

After `drive_resource_flow`, create a `Device` resource, bind the target as driver, raise a state-change event, deliver it, and acknowledge it. Use explicit capabilities only; do not mutate core internals.

- [ ] **Step 5: Update README**

Add driver binding and device event lifecycle to current scope, current behavior, event list, and expected supervisor trace. Do not add QEMU expected output because boot handoff does not exercise driver events in V0.

- [ ] **Step 6: Run supervisor test to verify green**

Run the same command. Expected: PASS.

- [ ] **Step 7: Commit**

```bash
git add crates/agent-supervisor/src crates/agent-supervisor/tests/supervisor_flow.rs crates/agent-kernel-x86_64/src/main.rs README.md
git commit -m "feat: drive agent driver events"
```

### Task 6: Final Validation And Publish

**Files:**
- Verify all modified files.

- [ ] **Step 1: Run formatting**

```bash
RUSTC="$(rustup which --toolchain nightly rustc)" RUSTDOC="$(rustup which --toolchain nightly rustdoc)" rustup run nightly cargo fmt --check
```

Expected: PASS.

- [ ] **Step 2: Run focused workspace tests**

```bash
RUSTC="$(rustup which --toolchain nightly rustc)" RUSTDOC="$(rustup which --toolchain nightly rustdoc)" rustup run nightly cargo test -p agent-kernel-core --tests
RUSTC="$(rustup which --toolchain nightly rustc)" RUSTDOC="$(rustup which --toolchain nightly rustdoc)" rustup run nightly cargo test -p agent-kernel --tests
RUSTC="$(rustup which --toolchain nightly rustc)" RUSTDOC="$(rustup which --toolchain nightly rustdoc)" rustup run nightly cargo test -p agent-supervisor --test supervisor_flow
```

Expected: PASS.

- [ ] **Step 3: Run full workspace tests**

```bash
RUSTC="$(rustup which --toolchain nightly rustc)" RUSTDOC="$(rustup which --toolchain nightly rustdoc)" rustup run nightly cargo test --workspace
```

Expected: PASS. If bootloader still reports `failed to get llvm tools: NotFound`, retry after ensuring the nightly sysroot `lib/rustlib/aarch64-apple-darwin/bin` directory contains `llvm-objcopy`; this was observed as a clean-worktree bootstrap issue before feature code changed.

- [ ] **Step 4: Run supervisor and QEMU paths**

```bash
RUSTC="$(rustup which --toolchain nightly rustc)" RUSTDOC="$(rustup which --toolchain nightly rustdoc)" rustup run nightly cargo run -p agent-supervisor
./scripts/run-qemu.sh
```

Expected: supervisor output includes the four driver labels. QEMU output remains the boot handoff sequence through `SUPERVISOR_HANDOFF_READY`.

- [ ] **Step 5: Run no_std forbidden symbol scan**

```bash
for crate in crates/agent-kernel-core crates/agent-kernel; do
  rg -n "Vec<|String|Box<|HashMap|Rc<|Arc<|std::|println!|format!|thread::|fs::|Tcp|Udp|async|await" "$crate/src" || true
done
```

Expected: no production-code hits beyond existing comments.

- [ ] **Step 6: Merge and push**

```bash
git status --short --branch
git -C /Users/ran/Desktop/agent-kernel checkout main
git -C /Users/ran/Desktop/agent-kernel pull --ff-only origin main
git -C /Users/ran/Desktop/agent-kernel merge --ff-only feature/agent-driver-binding-v0
git -C /Users/ran/Desktop/agent-kernel push origin main
```

Expected: `main` fast-forwards and pushes to the private GitHub repository.

## Self-Review

Spec coverage: the plan covers driver binding records, device event records,
capability checks, event ordering, atomic failure paths, facade syscalls,
supervisor formatting, documentation, no_std checks, and final publishing.

Marker scan: no unresolved-marker strings, open-ended edge-case instructions,
or missing command details remain.

Type consistency: `DriverBindingId`, `DeviceEventId`, `DeviceEventKind`,
`DeviceEventPayload`, `DeviceEventStatus`, `DriverBindingRecord`,
`DeviceEventRecord`, `bind_driver`, `raise_device_event`,
`deliver_device_event`, and `acknowledge_device_event` are used consistently
across tasks.
