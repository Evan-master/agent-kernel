# Agent Kernel

**English** | [简体中文](README.zh-CN.md)

Agent Kernel is an agent-native operating-system kernel written in Rust. Its
primary kernel objects are Agents, Resources, Capabilities, Intents, Tasks,
Events, Verification, and Rollback. The architecture is defined independently
of Linux, shell automation, and POSIX compatibility.

> **Development status:** active kernel development. The freestanding x86_64
> target boots in QEMU and executes isolated ring-3 Agent Capsules. The ABI and
> architecture are under active revision; production stability is planned for
> a later stage.

## System Model

Traditional operating systems give programs process, file, socket, and user
abstractions. An agent-first system needs a different control plane:

- An **Agent** is a kernel-visible execution and authority subject.
- A **Resource** is any kernel-managed object an Agent may control.
- A **Capability** says exactly which Agent may perform which operations on
  which Resource; authority can be attenuated and revoked.
- An **Intent** declares desired work, while a **Task** is its schedulable
  execution unit.
- **Verification** is separate from successful execution.
- **Checkpoint** and **Rollback** are first-class lifecycle operations.
- Every successful mutation produces deterministic audit evidence. Ordinary
  mutations append an ordered **Event**; Event archive commits advance a
  cryptographic checkpoint chain so a full log can release capacity safely.

There is no ambient superuser inside the native model. High authority is
possible, but it must be represented by explicit capabilities and remains
observable in the audit chain.

## Current Implementation

The reference BIOS/QEMU configuration boots directly on virtual hardware and
currently provides:

- a permanent GDT, TSS, IDT, ring-0/ring-3 boundary, and private Agent CR3 roots;
- eleven completed isolated native Agent contexts: two initial Workers, a
  Verifier, a Fault Worker, a Fault Handler, a Resource Manager, an Admission
  Supervisor, and four post-reclamation Runtime Service Workers executed in
  two sequential batches;
- kernel-selected FIFO dispatch with physical PIT timer preemption and full CPU
  frame ownership across resume;
- SHA-256-bound, fixed-size Agent Image Capsules with typed Worker, Verifier,
  FaultHandler, and Supervisor entry roles;
- a versioned register-only Agent Call ABI with no userspace pointers;
- blocking mailbox send/receive/acknowledge, recipient-owned acknowledged
  Message retirement, administrator-owned orphaned Message retirement, wakeup,
  cooperative yield, task results, target-scoped verification, and completion;
- containment of ring-3 `#UD`, `#GP`, and `#PF` faults while kernel-origin
  exceptions remain fatal;
- bounded fault-time reclamation of live private runtime memory before
  `TaskFaulted`: exact-Capability Resource retirement, leaf removal, physical
  frame zeroing and return, fixed-capacity evidence, and restartable CPU
  capture;
- the same bounded transaction on authenticated `CompleteTask`, with completion
  readiness preflight and ordered reclamation evidence attached to the
  completed CPU;
- complete ownership identity for four private page-table frames and seven
  content frames per native Agent, followed by terminal zeroing and transfer
  into a fixed-capacity reusable frame pool;
- an Agent-bound native address-space runtime service that owns allocation,
  exact P4/P3/P2/P1 reconstruction, CPU preparation, and runtime registration
  as one transactional admission;
- a fixed-capacity Runtime Admission object with root-scoped `Delegate`
  authorization, FIFO request preparation, generation-bound permits, bounded
  rejection causes, and atomic admission plus Task queueing;
- an independently configured Runtime Admission capacity, defaulted to the Task
  capacity for source compatibility; the x86 profile provisions 16 Admission
  slots for 12 Tasks, and terminal rejections permit monotonic-ID retries while
  retaining prior evidence until compaction;
- Agent Call 27 and a real ring-3 Admission Supervisor Capsule that creates four
  audited Runtime Admission requests across two rounds, blocks twice in its
  Mailbox, and retains one CPU/address-space context throughout both batches;
- Agent Call 28, which exposes the permit-bound requester only to an admitted
  context; each Runtime Service Worker validates the reply and uses that
  identity as its completion notification recipient;
- Agent Call 29, which lets an authenticated Supervisor compact an authorized
  terminal Runtime Admission prefix, return active capacity, invalidate stale
  permits, and emit one ordered audit Event per retired record;
- Agent Call 30, which lets an authenticated Supervisor compact an authorized
  terminal Task prefix, reject live references, preserve monotonic Task IDs,
  invalidate stale dispatch permits, and emit one ordered audit Event per
  retired Task;
- Agent Call 31, which lets an authenticated Supervisor compact an authorized
  terminal Intent prefix after active Task and Message references are gone,
  preserve monotonic Intent IDs, and emit one ordered audit Event per retired
  Intent;
- Agent Call 32, which lets an authenticated Supervisor retire one revoked
  Capability leaf after every live child, Task, Agent Entry, Runtime Admission,
  and unacknowledged Message reference is gone; retired child Resources can be
  cleaned through active ancestor `Rollback` authority;
- Agent Call 33, which lets an authenticated Supervisor retire one quiescent
  terminal Agent Entry after its native runtime context and every live kernel
  reference are gone; the dense Entry Store shifts deterministically while the
  Agent identity, terminal Task, delegated Capability, Image, and audit history
  remain intact;
- Agent Call 34, which lets the authenticated recipient retire one acknowledged
  Message, return its dense Store slot, preserve retained FIFO order and
  monotonic Message IDs, and emit complete kind and payload audit evidence;
- Agent Call 35, which lets an authenticated administrator retire one Pending
  Message addressed to a retired managed Agent through exact root-scoped
  `Delegate` authority, return its dense Store slot, and emit complete
  recipient, authority, kind, and payload evidence;
- Agent Call 36, which lets an authenticated administrator retire one terminal
  managed Agent record and its index-aligned execution context after every live
  non-Event reference is gone; a monotonic retirement high-water prevents
  historical Agent identities from being registered again;
- Agent Call 37, which lets an authenticated Supervisor retire one terminal
  Agent Image record through root-scoped `Rollback` cleanup authority after
  every Agent Entry, Runtime Admission, and native execution-context reference
  is gone; stable dense removal returns Image Store capacity while monotonic
  Image IDs preserve historical identity;
- Agent Call 38, which lets an authenticated Supervisor compact a contiguous
  inactive Waiter prefix through shared `Rollback` cleanup authority, return
  dense Store capacity, preserve monotonic Waiter IDs, and emit one complete
  audit Event per retired Waiter;
- Agent Call 39, which lets an authenticated Supervisor compact a contiguous
  recovered Fault prefix through shared `Rollback` cleanup authority, reject
  live Task or Message references, clear safe Task history pointers, preserve
  monotonic Fault IDs, and emit one complete audit Event per retired Fault;
- Agent Call 40, which lets a launched Supervisor use root-scoped `Rollback`
  authority to prepare and commit a dense Event prefix archive, retain a
  chained SHA-256 checkpoint, preserve monotonic Event sequence numbers, and
  recover capacity even when the live Event Log is full;
- Agent Call 41, which lets a launched Supervisor retire one terminal,
  unreferenced Resource record through active ancestor `Rollback` authority,
  return its dense Store slot, preserve monotonic Resource IDs, and emit a
  complete fixed-width receipt plus ordered audit Event;
- Agent Call 42, which lets a launched Supervisor revoke residual authority on
  a retired Resource through active ancestor `Rollback` authority, enabling
  leaf-first Capability cleanup before Resource record retirement;
- a 64-record architecture archive handoff that captures Events 1 through 64
  while all 357 live Event slots are occupied, verifies the checkpoint in
  ring 3, resumes execution at Event 358, and later merges archived and live
  iterators into the exact Event 1 through 383 transcript;
- a seven-slot Resource Store recovery proof that cleanup-revokes Capability
  13, compacts Capabilities 14, 13, and 26 in child-first order, retires
  Resource record 3, then creates monotonic Resource 8 in the returned slot;
- an x86 admission broker that verifies each permit-bound Capsule, drives the
  existing address-space service, commits semantic admission, and restores the
  physical runtime transaction if the semantic commit cannot proceed;
- four authenticated Worker completion notifications that wake the retained
  Supervisor call frame across two FIFO receive/acknowledge/retire rounds; two
  transient slots are reused by Messages 4 through 7 across both batches;
- a three-slot x86 Waiter Store proof: the Supervisor retires inactive Waiters
  1 through 3 after the first notification round, creates monotonic Waiter 4
  in a reclaimed slot for the second round, then retires it and leaves the
  active Store empty;
- a four-slot x86 Fault Store proof: the Fault Worker fills the Store with
  Faults 1 through 4, terminal Task compaction clears the live blockers, and
  the resident Supervisor retires the full Fault prefix through Agent Call 39,
  leaving the active Store empty while preserving all historical Events;
- two generation-bound Runtime Admission release batches that require verified,
  idle targets and preflight aggregate event capacity: batch one returns 22
  frames while the Supervisor retains eleven; the Supervisor later compacts
  released Admissions 1 and 2, and the terminal batch returns the final 33
  frames while retaining released Admissions 3 and 4;
- post-build cancellation that clears and atomically restores all eleven frames
  after duplicate runtime registration, followed by cross-batch physical reuse:
  Agents 13 and 14 consume the exact zeroed identities released by Agents 11
  and 10 while the Supervisor identity remains disjoint and resident;
- policy routing to a real ring-3 Fault Handler, followed by capability-gated
  retained-page repair and same-frame resume;
- a real ring-3 Resource Manager that creates a child Service through delegated
  `Act` authority, derives attenuated `Observe` authority for another Agent,
  revokes that direct child through `Delegate`, retires the Service through
  `Rollback`, declares a new `Act` Intent, creates its Task, and delegates that
  Task to a registered Agent with a kernel-issued task capability;
- a native Agent Manager protocol in the same ring-3 Capsule that registers
  Agent 9 under root-scoped `Delegate` authority, sends Pending Message 3,
  suspends, resumes, and retires the unlaunched identity, then retires the
  orphaned Message through Agent Call 35, retires the Agent 9 record/context
  pair through Agent Call 36, and registers Agent 15 into the reclaimed slot;
- a native Agent Image Manager protocol that retires disposable Worker Image 9
  through Agent Call 37, validates complete removal evidence, and later
  registers Image 15 into the reclaimed physical slot; the final dense Store
  holds the monotonic identities 1 through 8 and 10 through 15;
- a shared 16-frame runtime pool with deterministic allocation, full-page
  zeroing, and ownership records bound to Agent, Resource, MemoryCell, and
  allocation generation;
- physically backed runtime memory lifecycles for a compatibility page and
  kernel-selected regions of one to four pages, including ring-3 proof writes,
  first/last-page inspection, concurrent live mappings, deterministic
  first-fit hole reuse, leaf removal, and frame reclamation;
- a fixed-capacity ordered region-observation log that carries allocation
  identity and ring-3 proof values into terminal kernel evidence;
- a kernel-authorized Driver flow from UART interrupt through endpoint lookup,
  immutable HAL request, Port I/O, result recording, and invocation completion.

The reference validation profile enforces these deterministic invariants:

| Evidence | Count |
| --- | ---: |
| Registered Agent Events | 15 |
| Final resident Agent records | 14 |
| Retired Agent record/context pairs | 1 |
| Reused Agent record/context slots | 1 |
| Agent record retirement Events | 1 |
| Agent retirement high-water | 9 |
| Registered Agent Image Events | 15 |
| Final resident Agent Image records | 14 |
| Retired Agent Image records | 1 |
| Reused Agent Image record slots | 1 |
| Agent Image record retirement Events | 1 |
| Native ring-3 completions | 11 |
| Kernel-selected dispatches | 35 |
| Resource Manager Agent Calls | 34 |
| Resource Manager Agent/kernel address-space switches | 68 |
| Admission Supervisor Agent Calls | 38 |
| Admission Supervisor Agent/kernel address-space switches | 76 |
| Runtime Service Worker Agent Calls | 20 |
| Runtime Service Worker Agent/kernel address-space switches | 40 |
| Physical quantum expiries | 15 |
| Task store capacity | 12 |
| Compacted terminal Tasks | 6 |
| Active Tasks after prefix compaction | 6 |
| Task compaction Events | 6 |
| Intent store capacity | 12 |
| Compacted terminal Intents | 6 |
| Active Intents after prefix compaction | 6 |
| Intent compaction Events | 6 |
| Agent Entry store capacity | 14 |
| Retired Agent Entry records | 2 |
| Active Agent Entry records after retirement | 11 |
| Agent Entry retirement Events | 2 |
| Runtime Admission store capacity | 16 |
| Runtime Admission requests | 4 |
| Runtime Admission commits | 4 |
| Runtime Admission requester discoveries | 4 |
| Runtime Admission releases | 4 |
| Runtime Admission compaction Events | 2 |
| Retained terminal Runtime Admission records | 2 |
| Worker completion notifications | 4 |
| Message store capacity | 4 |
| Retained acknowledged boot-flow Messages | 2 |
| Retired completion Messages | 4 |
| Recipient-owned Message retirement Events | 4 |
| Administrator-owned orphaned Message retirement Events | 1 |
| Resident Supervisor Mailbox waits | 2 |
| Resident Supervisor Mailbox wakes | 2 |
| Waiter store capacity | 3 |
| Compacted inactive Waiter records | 4 |
| Waiter compaction Events | 4 |
| Reused Waiter slots | 1 |
| Final resident Waiter records | 0 |
| Contained Agent faults | 4 |
| Fault store capacity | 4 |
| Compacted recovered Fault records | 4 |
| Fault compaction Events | 4 |
| Final resident Fault records | 0 |
| Fault-owned live regions reclaimed | 1 |
| Fault-owned physical frames reclaimed | 2 |
| Completion-owned live regions reclaimed | 1 |
| Completion-owned physical frames reclaimed | 3 |
| Rejected native admission cancellations | 1 |
| Frames restored by admission cancellation | 11 |
| Native address-space reclamation completions | 11 |
| Cumulative terminal private-frame returns | 121 |
| Final zeroed private address-space frame pool | 66 |
| Resource store capacity | 7 |
| Occupied Resource records after retirement and reuse | 7 |
| Retired Resource records | 1 |
| Reused Resource slots | 1 |
| Capability store capacity | 26 |
| Occupied Capability slots after compaction and reuse | 26 |
| Compacted Capability records | 3 |
| Reused Capability slots | 3 |
| MemoryCells after Manager execution | 5 |
| Shared runtime frames returned and zeroed | 16 |
| Live Event Log capacity | 357 |
| Live Event occupancy before archive | 357 |
| Events in the architecture archive | 64 |
| Final live Event occupancy | 319 |
| Final next Event sequence | 384 |
| Retained Event archive checkpoints | 1 |
| Ordered kernel events after Driver completion | 383 |

`scripts/run-qemu.sh` validates every event in order and rejects missing
markers, extra events, an unexpected QEMU exit status, or any fail-closed boot
path.

The two persistent Supervisor Capsules are also frozen as independent binary
artifacts:

| Capsule | Agent Calls | Address-space switches | Capsule bytes | SHA-256 |
| --- | ---: | ---: | ---: | --- |
| Resource Manager | 34 | 68 | 3,195 | `d86e0918da3eb102ba24d382812c60cf005829888b508817bbd51ea34925af9e` |
| Admission Supervisor | 38 | 76 | 3,797 | `12d8f989d16454ce12d6de369033f00d70717ba8d7abee400168ca5610047b0b` |

The generated Rust bytes match independently assembled machine code, and each
complete Capsule occurs exactly once in the release ELF.

## Architecture

```mermaid
flowchart TB
    Capsule["Verified Agent Capsule"] --> Ring3["Isolated ring-3 Agent"]
    Ring3 -->|"Agent Call / interrupt / fault"| X86["x86_64 boundary adapter"]
    X86 --> Facade["agent-kernel syscall facade"]
    Facade --> Core["agent-kernel-core deterministic model"]
    Core --> Stores["Fixed-capacity object stores"]
    Core --> Events["Bounded live Event Log"]
    Core --> ArchiveHead["Event archive checkpoint chain head"]
    Core --> Scheduler["Task and Driver scheduler"]
    Scheduler --> Runtime["Native CPU runtime"]
    Runtime --> X86
    FramePool["Zeroed private frame pool"] <--> AddressService["Address-space runtime service"]
    Broker["Native Runtime Admission broker"] --> AddressService
    Core -->|"Generation-bound permit"| Broker
    AddressService --> Runtime
    X86 --> Archive["Bounded external Event archive"]
    Core --> HAL["Immutable HAL request"]
    HAL --> Device["Architecture or host device backend"]
    Supervisor["ring-3 Admission Supervisor"] -->|"Agent Calls 27, 29-42"| X86
    Workers["Admitted ring-3 Workers"] -->|"Agent Call 28"| X86
    Workers -->|"Notify / Mailbox"| Supervisor
```

The kernel remains deterministic and compact. A userspace Supervisor owns LLM
inference, prompts, remote model calls, and high-level planning; kernel space
owns deterministic execution and authority primitives.

## Workspace

| Crate | Responsibility |
| --- | --- |
| `agent-kernel-core` | `no_std` AgentOS object model, authorization, lifecycle, scheduler, and events |
| `agent-kernel` | `no_std` syscall-style facade over the core |
| `agent-kernel-hal` | Immutable, kernel-authorized device request contract |
| `agent-kernel-boot` | Deterministic bootstrap handoff and fixed capacities |
| `agent-kernel-x86_64` | Freestanding x86_64 boot, isolation, interrupts, faults, Agent Calls, and QEMU validation |
| `agent-kernel-image` | Host utility that builds the BIOS disk image |
| `agent-supervisor` | Host-side userspace simulation and virtual device backend |

All kernel stores use fixed capacities. The core and facade are heap-free and
host-independent, with explicit state ownership and deterministic inputs.

## Agent Call ABI

Agent Calls cross the ring-3 boundary through a fixed register frame. Every
mutating request is authenticated against scheduler-owned Agent, Task, Image,
and nonce state before it reaches the facade.

| Operation | ID | Purpose |
| --- | ---: | --- |
| `DescribeContext` | 1 | Establish trusted execution identity and nonce |
| `Yield` | 2 | Cooperatively return the running Task to the queue |
| `CompleteTask` | 3 | Reclaim live private memory and complete the authenticated Task |
| `SubmitTaskResult` | 4 | Store a fixed-width Task result |
| `InspectTaskResult` | 5 | Inspect one authorized target result |
| `VerifyTask` | 6 | Commit target-scoped verification |
| `SendMessage` | 7 | Send a typed kernel-object message |
| `ReceiveMessage` | 8 | Receive or atomically enter mailbox wait |
| `AcknowledgeMessage` | 9 | Acknowledge the received message |
| `CreateResource` | 10 | Create a child Resource through explicit parent authority |
| `RetireResource` | 11 | Retire a Resource through its `Rollback` capability |
| `DeriveCapability` | 12 | Attenuate source authority for another registered Agent |
| `RevokeDerivedCapability` | 13 | Revoke one direct child through its `Delegate` source |
| `DeclareIntent` | 14 | Declare typed work through explicit Resource authority |
| `CreateTask` | 15 | Create a Task from an owned declared Intent |
| `DelegateTask` | 16 | Delegate a created Task and issue task-scoped authority |
| `RegisterManagedAgent` | 17 | Register an unlaunched Agent under Resource-scoped management authority |
| `SuspendManagedAgent` | 18 | Suspend a quiescent managed Agent |
| `ResumeManagedAgent` | 19 | Reactivate a suspended managed Agent |
| `RetireManagedAgent` | 20 | Commit the terminal state of a quiescent managed Agent |
| `AllocateMemoryPage` | 21 | Map one kernel-selected private page under an owned Memory Resource |
| `InspectMemoryPage` | 22 | Audit and return the first fixed-width value from the mapped page |
| `ReleaseMemoryPage` | 23 | Retire its Memory Resource, remove the leaf, and clear the frame |
| `AllocateMemoryRegion` | 24 | Map a kernel-selected region of one to four pages under an owned Memory Resource |
| `InspectMemoryRegion` | 25 | Audit and return the first value from the first and last mapped pages |
| `ReleaseMemoryRegion` | 26 | Retire its Memory Resource, remove every leaf, clear every frame, and return the region to the pool |
| `RequestRuntimeAdmission` | 27 | Request audited native runtime admission for one accepted, unqueued target Task |
| `DiscoverRuntimeAdmission` | 28 | Return the kernel-owned requester bound to the current admitted context |
| `CompactRuntimeAdmissions` | 29 | Retire an authorized terminal prefix from the active admission queue |
| `CompactTasks` | 30 | Retire an authorized terminal prefix from the active Task store |
| `CompactIntents` | 31 | Retire an authorized terminal prefix from the active Intent store |
| `CompactCapability` | 32 | Retire one authorized revoked leaf from the sparse Capability store |
| `RetireAgentEntry` | 33 | Retire one authorized quiescent terminal entry from the dense Agent Entry store |
| `RetireMessage` | 34 | Retire one acknowledged Message owned by the authenticated recipient |
| `RetireOrphanedMessage` | 35 | Retire one Pending Message addressed to an authorized retired managed Agent |
| `RetireAgentRecord` | 36 | Retire one unreferenced terminal managed Agent record and aligned execution context |
| `RetireAgentImageRecord` | 37 | Retire one unreferenced terminal Agent Image record from the dense Image Store |
| `CompactWaiters` | 38 | Retire an authorized inactive prefix from the dense Waiter Store |
| `CompactFaults` | 39 | Retire an authorized recovered prefix from the dense Fault Store |
| `ArchiveEvents` | 40 | Commit an authenticated Event prefix handoff and return its chained digest |
| `RetireResourceRecord` | 41 | Retire one unreferenced terminal Resource record from the dense Resource Store |
| `RevokeCapabilityForCleanup` | 42 | Revoke residual authority on a retired Resource through active ancestor authority |

The native Resource creation ABI accepts AgentOS-oriented Workspace, Memory,
Service, Network, and Device kinds. Resource retirement replies also encode
File and Process records already retained by Core. Unknown creation kinds,
unknown operation bits, zero handles, stale nonces, wrong identities, and
non-zero reserved registers fail closed.
The Task Manager ABI accepts the five native Intent kinds and explicit optional
or required verification policy codes. Agent management requires an active,
root-scoped `Delegate` Capability on the identity's management Resource. The
target must have an idle execution context, no launch entry, and no active
assigned Task. Runtime memory calls accept only Capability and kernel-object
handles plus a bounded page count. Virtual addresses, physical frames, access
flags, and byte lengths remain kernel-selected.

Runtime admission requires an authenticated Supervisor entry and an active,
root-scoped `Delegate` Capability on the target Task Resource. The kernel binds
the request to the target Agent, Task, verified Image, and Resource. The x86
broker receives only a generation-checked permit and commits queue visibility
after physical registration succeeds. The broker carries the permit requester
into the admitted CPU context; operation 28 returns it through a canonical,
authenticated, read-only reply. Operation 29 requires `Delegate` authority for
every selected Resource, compacts only a contiguous `Rejected` or `Released`
prefix, preserves monotonic IDs, and records every retired identity in the
Event log. Operation 30 requires root-scoped `Rollback` authority for every
selected Task Resource. It accepts only a contiguous `Verified`/`Fulfilled` or
`Cancelled`/`Cancelled` prefix with no active queue, execution, waiter,
admission, Namespace, or Message reference. Successful compaction advances the
Task generation, preserves monotonic IDs, and records the complete retired Task
identity in ordered Events. Operation 31 requires root-scoped `Rollback`
authority for every selected Intent Resource. It accepts only a contiguous
`Fulfilled` or `Cancelled` prefix after all active Task and unacknowledged
Message references are gone, preserves monotonic Intent IDs, and records the
original kind, Resource, owner, and verification requirement in ordered Events.
Operation 32 requires an authenticated Supervisor and active root-scoped
`Rollback` authority. It accepts one revoked Capability with no retained child
or live kernel-object reference, clears exactly one sparse slot, preserves
monotonic IDs, and emits a `CapabilityCompacted` Event. An active target
Resource requires exact scope; a retired target Resource accepts authority from
an active ancestor in its immutable Resource chain. Operation 33 requires an
authenticated active Supervisor and root-scoped `Rollback` cleanup authority.
The target must have an idle execution context, terminal retained Task or
Intent state, no native runtime context, and no live queue, Waiter, Admission,
received Message, Fault Handler, or Driver Binding reference. Successful
retirement removes one dense Entry record, preserves the referenced terminal
objects, permits a later relaunch of the same Agent, and emits an
`AgentEntryRetired` Event carrying the complete launch identity.
Operation 34 remains in the Agent identity domain used by Send, Receive, and
Acknowledge. The authenticated caller must exactly match the Message recipient,
and only `Acknowledged` records with no live Namespace binding may retire.
Dense removal preserves retained order, the next Message ID remains monotonic,
and `MessageRetired` records the sender, recipient, kind, and every typed
payload reference before the slot returns to the fixed Store.
Operation 35 requires an active authenticated caller and an active, exact,
root-scoped `Delegate` Capability on the retired recipient's recorded
management Resource. Only `Pending` Messages addressed to managed Agents in
`Retired` state qualify. A live Namespace binding blocks removal. Successful
retirement preserves dense order and monotonic Message IDs, then emits
`OrphanedMessageRetired` with the recipient, authority, operation, kind, and
every typed payload reference.
Operation 36 requires an active caller, an exact active root-scoped `Delegate`
Capability on the target's management Resource, a `Retired` target, an idle
execution context, no native runtime context, and no retained non-Event Store
reference. The transaction removes the dense Agent record and its aligned
execution context together, advances the retirement high-water, and emits
`AgentRecordRetired`. Later registration rejects zero identities and every
identity at or below that high-water, so historical Events cannot alias a new
Agent.
Operation 37 requires an authenticated active Supervisor and root-scoped
`Rollback` cleanup authority. The target Image must be `Retired`, with no
Agent Entry, Runtime Admission, current caller, parked runtime, completed
report, or faulted report carrying its identity. Active Image Resources require
exact authority scope; retired Image Resources accept active ancestor scope.
Successful retirement removes one dense Image record in stable order, leaves
the monotonic Image allocator unchanged, and emits `AgentImageRecordRetired`
with the complete Image metadata, actor, and authorizing Capability.
Operation 38 requires an authenticated active Supervisor and shared cleanup
authorization for every selected Waiter Resource. Every record in the selected
prefix must be inactive. Active Resources require exact `Rollback` scope;
retired Resources accept active ancestor scope. The transaction preflights
aggregate Event capacity, removes the prefix in stable order, preserves the
monotonic Waiter allocator, and emits `WaiterCompacted` with the original
Agent, Task, Resource, signal, Waiter kind, and identity.
Operation 39 requires an authenticated active Supervisor and shared cleanup
authorization for every selected Fault Resource. A `Faulted` Task or a
non-acknowledged Message that references the selected prefix blocks the whole
transaction. Active Resources require exact `Rollback` scope; retired
Resources accept active ancestor scope. After aggregate Event preflight, the
transaction clears safe Task `last_fault` pointers, removes the prefix in
stable order, preserves the monotonic Fault allocator, and emits
`FaultCompacted` with the original Agent, Task, Resource, Fault kind, detail,
identity, actor, and authority.
Operation 40 requires a launched Supervisor and an active, task-unscoped,
root-scoped `Rollback` Capability. Preparation selects a non-empty contiguous
live prefix without mutation and computes a canonical SHA-256 proposal over
every Event field, the archive generation, and the previous digest. Commit
recomputes and compares the full proposal before removing the prefix in stable
order. The next Event sequence remains monotonic, and a dedicated checkpoint
records the actor, authority, root Resource, range, count, predecessor digest,
and resulting digest. The x86 reply returns the range, count, and four
little-endian digest words without userspace pointers; the generation remains
part of the retained Core checkpoint.
Operation 41 requires a launched Supervisor, a `Retired` target Resource, and
active ancestor `Rollback` authority. The transaction rejects every retained
non-Event reference, reserves one Event slot, removes the target in stable
dense order, leaves the monotonic Resource allocator unchanged, and returns
the complete removed record with `ResourceRecordRetired` audit evidence. The
x86 reply carries the Resource identity, stable kind code, optional parent,
and optional owner in fixed registers.
Operation 42 requires a launched Supervisor and active, task-unscoped
`Rollback` authority scoped to an ancestor in the retired target's immutable
Resource chain. It revokes one residual Capability after complete validation
and emits `CapabilityRevoked` with the cleanup authority and `Rollback`
operation. Existing leaf-first Capability compaction then returns the sparse
slot before operation 41 removes the Resource record.

## Quick Start

### Requirements

- Rust installed through `rustup`;
- the repository's pinned nightly toolchain, `rust-src`, LLVM tools, and
  `x86_64-unknown-none` target (installed automatically from
  `rust-toolchain.toml` by rustup-managed Cargo);
- `qemu-system-x86_64` for the freestanding x86_64 validation target.

On macOS, QEMU can be installed with:

```bash
brew install qemu
```

### Build And Test

```bash
git clone https://github.com/Evan-master/agent-kernel.git
cd agent-kernel

cargo fmt --check
cargo test --workspace
cargo run -p agent-supervisor
```

### Run The x86_64 Validation Target

```bash
scripts/run-qemu.sh
scripts/run-qemu.sh --release
```

The scripts build the freestanding target, create a BIOS image, start QEMU,
validate the complete serial transcript, require exactly 383 events, and treat
the kernel's debug-exit status as part of the contract. A successful run
includes these proof lines:

```text
AGENT_KERNEL_NATIVE_FAULT_MEMORY_RECLAIMED_OK
AGENT_KERNEL_NATIVE_COMPLETION_MEMORY_RECLAIMED_OK
AGENT_KERNEL_RUNTIME_FRAME_POOL_RELEASED_OK
AGENT_KERNEL_NATIVE_ADDRESS_SPACE_RECLAIMED_OK
AGENT_KERNEL_NATIVE_ADDRESS_SPACE_FRAME_POOL_OK
AGENT_KERNEL_NATIVE_ADDRESS_SPACE_RUNTIME_CANCEL_OK
AGENT_KERNEL_NATIVE_ADDRESS_SPACE_ALLOCATED_OK
AGENT_KERNEL_NATIVE_ADDRESS_SPACE_REBUILT_OK
AGENT_KERNEL_NATIVE_ADDRESS_SPACE_RUNTIME_BATCH_OK
AGENT_KERNEL_NATIVE_ADDRESS_SPACE_RUNTIME_CONCURRENCY_OK
AGENT_KERNEL_RUNTIME_ADMISSION_CAPACITY_OK
AGENT_KERNEL_NATIVE_FAULT_STORE_FULL_OK
AGENT_KERNEL_AGENT_CALL_RUNTIME_ADMISSION_REQUEST_OK
AGENT_KERNEL_AGENT_CALL_RUNTIME_ADMISSION_DISCOVERY_OK
AGENT_KERNEL_AGENT_CALL_RUNTIME_ADMISSION_COMPACTION_OK
AGENT_KERNEL_TASK_PREFIX_VERIFIED_OK
AGENT_KERNEL_AGENT_CALL_TASK_COMPACTION_OK
AGENT_KERNEL_AGENT_CALL_FAULT_COMPACTION_OK
AGENT_KERNEL_AGENT_CALL_INTENT_COMPACTION_OK
AGENT_KERNEL_AGENT_CALL_AGENT_ENTRY_RETIREMENT_OK
AGENT_KERNEL_AGENT_CALL_MESSAGE_RETIREMENT_OK
AGENT_KERNEL_AGENT_CALL_ORPHANED_MESSAGE_RETIREMENT_OK
AGENT_KERNEL_AGENT_CALL_AGENT_RECORD_RETIREMENT_OK
AGENT_KERNEL_AGENT_CALL_AGENT_IMAGE_RECORD_RETIREMENT_OK
AGENT_KERNEL_AGENT_CALL_CAPABILITY_COMPACTION_OK
AGENT_KERNEL_AGENT_CALL_WAITER_COMPACTION_OK
AGENT_KERNEL_AGENT_CALL_EVENT_ARCHIVE_OK
AGENT_KERNEL_AGENT_CALL_CAPABILITY_CLEANUP_REVOCATION_OK
AGENT_KERNEL_AGENT_CALL_RESOURCE_RECORD_RETIREMENT_OK
AGENT_KERNEL_NATIVE_RUNTIME_ADMISSION_REQUEST_OK
AGENT_KERNEL_NATIVE_RUNTIME_ADMISSION_RESIDENT_WAIT_OK
AGENT_KERNEL_NATIVE_RUNTIME_ADMISSION_NOTIFICATION_OK
AGENT_KERNEL_NATIVE_MESSAGE_RETIREMENT_OK
AGENT_KERNEL_NATIVE_ORPHANED_MESSAGE_RETIREMENT_OK
AGENT_KERNEL_NATIVE_AGENT_RECORD_RETIREMENT_OK
AGENT_KERNEL_NATIVE_AGENT_IMAGE_RECORD_RETIREMENT_OK
AGENT_KERNEL_NATIVE_RUNTIME_ADMISSION_SUPERVISOR_OK
AGENT_KERNEL_NATIVE_RUNTIME_ADMISSION_COMMIT_OK
AGENT_KERNEL_NATIVE_RUNTIME_ADMISSION_RELEASE_OK
AGENT_KERNEL_NATIVE_ADDRESS_SPACE_PARTIAL_RECLAIM_OK
AGENT_KERNEL_NATIVE_RUNTIME_ADMISSION_REPEAT_OK
AGENT_KERNEL_NATIVE_RUNTIME_ADMISSION_COMPACTION_OK
AGENT_KERNEL_NATIVE_TASK_COMPACTION_OK
AGENT_KERNEL_NATIVE_FAULT_COMPACTION_OK
AGENT_KERNEL_NATIVE_INTENT_COMPACTION_OK
AGENT_KERNEL_NATIVE_AGENT_ENTRY_RETIREMENT_OK
AGENT_KERNEL_NATIVE_CAPABILITY_COMPACTION_OK
AGENT_KERNEL_NATIVE_WAITER_SLOT_REUSE_OK
AGENT_KERNEL_NATIVE_WAITER_COMPACTION_OK
AGENT_KERNEL_NATIVE_EVENT_LOG_FULL_OK
AGENT_KERNEL_NATIVE_EVENT_ARCHIVE_OK
AGENT_KERNEL_NATIVE_EVENT_ARCHIVE_REPLAY_OK
AGENT_KERNEL_NATIVE_CAPABILITY_CLEANUP_REVOCATION_OK
AGENT_KERNEL_NATIVE_RESOURCE_RECORD_RETIREMENT_OK
AGENT_KERNEL_NATIVE_RESOURCE_STORE_REUSE_OK
AGENT_KERNEL_NATIVE_ADDRESS_SPACE_REUSE_EXECUTION_OK
AGENT_KERNEL_NATIVE_ADDRESS_SPACE_REUSED_RECLAIMED_OK
AGENT_KERNEL_NATIVE_AGENT_IMAGE_SLOT_REUSE_OK
AGENT_KERNEL_NATIVE_RESOURCE_MANAGER_AGENT_OK
AGENT_KERNEL_NATIVE_CAPABILITY_MANAGER_OK
AGENT_KERNEL_NATIVE_TASK_MANAGER_OK
AGENT_KERNEL_NATIVE_AGENT_MANAGER_OK
AGENT_KERNEL_NATIVE_MEMORY_PAGE_MANAGER_OK
AGENT_KERNEL_NATIVE_MEMORY_REGION_MANAGER_OK
AGENT_KERNEL_NATIVE_MEMORY_CONCURRENCY_OK
AGENT_KERNEL_DRIVER_INVOCATION_FLOW_OK
event[340] fault_compacted
event[341] fault_compacted
event[342] fault_compacted
event[343] fault_compacted
event[354] capability_revoked
event[355] capability_compacted
event[356] capability_compacted
event[357] capability_compacted
event[358] resource_record_retired
event[359] resource_created
event[360] capability_granted
event[361] capability_derived
event[362] capability_derived
event[383] driver_invocation_completed
SUPERVISOR_HANDOFF_READY
```

## Authority And Failure Model

- Resource access always flows through an explicit capability.
- Task-scoped capabilities cannot silently become generic Resource authority.
- Derived authority cannot exceed its source and is invalidated by ancestor
  revocation.
- Architecture adapters route every mutation through public facade methods.
- Capacity checks occur before multi-record mutations so failures remain atomic.
- Compacted lifecycle history remains queryable through Events, while retained
  terminal objects reject future active use.
- Malformed Capsules, calls, CPU frames, event sequences, or physical ownership
  evidence terminate the validation run under the fail-closed policy.

High-authority Agents receive explicit, composable, revocable, and auditable
authority.

## Implemented And Planned

### Implemented

- Agent, Resource, Capability, Intent, Task, Action, Observation, Verification,
  Checkpoint, Rollback, Message, Fault, Driver, Memory Cell, Namespace, and
  Event primitives;
- capability grant, attenuation, task delegation, source-revocation
  propagation, authenticated direct-child revocation, resource ownership, and
  retirement;
- fixed-capacity scheduling, wait/wake, mailbox IPC, fault policy, image
  verification, semantic Runtime Admission, and Driver invocation lifecycle;
- freestanding x86_64 isolation, timer preemption, fault containment/recovery,
  native Resource, Capability, Intent, Task, managed Agent, shared physical
  frame-pool, compatibility-page, and multi-page memory-region lifecycle calls;
- deterministic fault-time retirement of live Memory Resources, private leaf
  removal, frame zeroing and return, bounded reclamation evidence, and restart
  after cleanup;
- authenticated completion-time retirement of live Memory Resources through
  the same fixed-capacity cleanup transaction;
- complete private page-table/content ownership tracking and terminal
  reclamation of six native address spaces into a 66-frame zeroed pool;
- Agent-bound, generation-checked eleven-frame allocation from that pool and a
  transactional runtime service spanning private hierarchy reconstruction,
  CPU preparation, and native runtime registration;
- a ring-3 Admission Supervisor, authenticated Agent Call 27 and Calls 29
  through 42,
  independently configured fixed-capacity admission records, terminal retry,
  generation-bound permits, requester-bound admitted contexts, and a broker
  that connects audited semantic requests to the physical runtime service;
- resident Supervisor Mailbox waiting across two admission and execution
  batches, authenticated Worker notifications, FIFO acknowledgement and
  recipient-owned Message retirement, two-slot reuse across batches, partial
  Worker reclamation, and final three-address-space reclamation;
- opaque, generation-bound release batches that link verified idle Tasks to
  post-reclamation `RuntimeAdmissionReleased` records and ordered kernel events;
- authorized terminal-prefix compaction with FIFO retention, monotonic IDs,
  stale-permit invalidation, active-capacity reuse, and per-record Events;
- authorized Task-prefix compaction with terminal Intent consistency, complete
  active-reference preflight, queue cleanup on cancellation, monotonic IDs,
  generation-bound dispatch permits, and per-Task Events;
- authorized Intent-prefix compaction with terminal-state checks, active Task
  and Message reference preflight, monotonic IDs, and complete per-Intent
  Events;
- authorized sparse Capability compaction with leaf-first ordering, live
  reference preflight, retired-Resource ancestor authority, monotonic IDs, hole
  reuse, and complete per-Capability Events;
- Supervisor-only residual Capability revocation on retired Resources through
  active ancestor `Rollback` authority, followed by leaf-first sparse cleanup;
- authorized dense Resource record retirement with terminal-state and complete
  non-Event reference preflight, active ancestor authority, monotonic IDs,
  stable retained order, fixed-width receipts, slot reuse, and complete Event
  evidence;
- authorized dense Agent Entry retirement with terminal-scope checks, native
  runtime and live-reference preflight, retired-Resource ancestor authority,
  stable retained-record order, same-identity relaunch, and complete retirement
  Events;
- recipient-owned acknowledged Message retirement with Namespace-reference
  preflight, dense-order preservation, monotonic IDs, slot reuse, and complete
  kind and typed-payload Event evidence;
- administrator-owned orphaned Message retirement for Pending delivery to a
  retired managed Agent, with exact root-scoped `Delegate` authorization,
  Namespace-reference preflight, atomic dense removal, monotonic IDs, and
  complete typed Event evidence;
- paired Agent record and execution-context retirement with exact root-scoped
  `Delegate` authorization, complete non-Event reference preflight, atomic
  dense removal, monotonic identity high-water, slot reuse, and complete Event
  evidence;
- authorized dense Agent Image record retirement with lifecycle and strict
  semantic/native-reference preflight, retired-Resource ancestor authority,
  monotonic Image IDs, stable retained order, physical slot reuse, and complete
  metadata Event evidence;
- authorized inactive Waiter-prefix compaction with aggregate Event preflight,
  shared Resource cleanup authority, stable retained order, monotonic Waiter
  IDs, bounded slot reuse, and complete per-record Event evidence;
- authorized recovered Fault-prefix compaction with active Task and Message
  reference preflight, safe Task-history cleanup, shared Resource cleanup
  authority, stable retained order, monotonic Fault IDs, full Store recovery,
  and complete per-record Event evidence;
- two-phase Event prefix archival with canonical full-field SHA-256 encoding,
  root Supervisor authorization, stable live-prefix release, monotonic Event
  sequencing, chained checkpoints, a bounded x86 handoff buffer, and exact
  archived/live replay through Event 383;
- complete rollback after rejected post-build admission, plus concurrent
  ownership, FIFO ring-3 execution, semantic verification, partial reclamation,
  and exact cross-batch frame reuse for four Runtime Service Workers;
- a fixed 2 MiB guarded kernel boot stack for the 383-event reference profile.

### Planned

- dynamic page-table growth beyond the fixed private hierarchy;
- durable crash-consistent Event archive storage, signed receipts, and remote
  transparency logs;
- SMP scheduling, multi-core synchronization, or hardware TLB shootdown;
- general storage, networking, graphics, USB, or physical hardware support;
- an Agent package/application format beyond the current bounded Capsule format;
- a production userspace Supervisor, model runtime, or policy planner;
- POSIX/Linux/Windows compatibility layers;
- production security hardening, formal verification, or stable ABI guarantees.

See the current [Resource Record Retirement design](docs/superpowers/specs/2026-07-19-resource-record-retirement-v1-design.md)
and [implementation plan](docs/superpowers/plans/2026-07-19-resource-record-retirement-v1.md)
for the latest milestone contract. Earlier design records remain under
`docs/superpowers/specs/`.

## Contributing

Before changing code, read [AGENTS.md](AGENTS.md). Contributions must preserve
the kernel architecture and validation contracts. In particular:

- keep the native model Agent-first and confine POSIX support to compatibility
  layers;
- preserve `no_std`, determinism, fixed-capacity storage, and explicit events;
- add failing tests before new runtime behavior;
- gate Resource-scoped privileged mutations with explicit Capabilities and
  identity-domain operations with exact caller authentication;
- run the workspace, Supervisor, and relevant QEMU validation before publishing.

## License

[MIT](LICENSE)
