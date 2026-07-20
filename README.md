<h1 align="center"><code>AGENT KERNEL</code></h1>

<p align="center">
  Agent-native authority, isolated execution, deterministic evidence.
</p>

<p align="center">
  <strong>English</strong> / <a href="README.zh-CN.md">简体中文</a>
</p>

<p align="center">
  <img alt="Rust nightly" src="https://img.shields.io/badge/Rust-nightly-111111?logo=rust&logoColor=white">
  <img alt="no_std" src="https://img.shields.io/badge/core-no__std-238636">
  <img alt="x86_64" src="https://img.shields.io/badge/target-x86__64--unknown--none-0969da">
  <img alt="QEMU" src="https://img.shields.io/badge/QEMU-events_1..396-f97316">
  <img alt="MIT" src="https://img.shields.io/badge/license-MIT-d0d7de">
</p>

```text
$ scripts/run-qemu.sh --release

[boot]       AGENT_KERNEL_QEMU_BOOT_OK
[isolation]  AGENT_KERNEL_MULTI_AGENT_ISOLATION_OK
[agents]     AGENT_KERNEL_HETEROGENEOUS_AGENT_EXECUTION_OK
[namespace]  AGENT_KERNEL_AGENT_CALL_NAMESPACE_PATH_OK
[audit]      AGENT_KERNEL_NATIVE_EVENT_ARCHIVE_REPLAY_OK
[event:396]  driver_invocation_completed
[handoff]    SUPERVISOR_HANDOFF_READY
```

```text
project  : active kernel development
abi      : 0.1 / unstable
core     : no_std / fixed capacity / heap free
machine  : x86_64 / BIOS / ring 0 + ring 3
proof    : tests + QEMU transcript + ELF audit
```

[`MODEL`](#01--native-model) / [`MACHINE`](#02--machine-path) /
[`ABI`](#03--agent-call-abi) / [`PROOF`](#06--verified-profile) /
[`BOOT`](#07--build--boot) / [`ROADMAP`](#09--roadmap)

## 00 / System Signal

| Channel | Kernel contract |
| --- | --- |
| Identity | Every call is bound to the scheduled Agent, Task, Image, and Nonce |
| Authority | Every Resource operation requires an explicit Capability |
| Mutation | Every successful state transition emits an ordered Event |
| Recovery | Checkpoint, Rollback, fault routing, repair, and restart |
| Isolation | Per-Agent CR3 roots and ring-3 execution contexts |
| I/O | Authorized HAL requests reach native interrupt and port paths |

## 01 / Native Model

```text
AGENT --presents--> CAPABILITY --controls--> RESOURCE
  |                                      |
  +--------------- emits EVENT <--------+
```

```text
identity     Agent / Task / Image / Execution Context
authority    Capability / Scope / Operations / Delegation
work         Intent / Action / Observation / Verification
recovery     Checkpoint / Rollback / Fault / Restart
structure    Workspace / Namespace / Entry / Revision
evidence     Event / Archive Digest / Replay
```

| Primitive | Responsibility |
| --- | --- |
| `Agent` | Authenticated authority subject with schedulable state |
| `Capability` | Scoped operations over one Resource; derivable and revocable |
| `Intent` | Typed declaration of desired work |
| `Task` | Schedulable work bound to delegated authority |
| `Verification` | Independent trust transition after execution |
| `Checkpoint` | Explicit recovery point governed by Rollback authority |
| `Event` | Deterministic evidence for successful mutation |
| `Namespace` | Revisioned binding, explicit Workspace Mount, bounded path |

```text
model runtime / prompts / planning / external adapters  -> user space
identity / authority / scheduling / isolation / audit   -> kernel space
```

## 02 / Machine Path

```text
RING 3   Verified Capsule -> Agent -> int 0x90 / IRQ / Fault
                                  |
---------------- privilege boundary ---------------------
                                  v
RING 0   x86_64 entry -> ABI decode -> auth -> no_std facade
                                  |
                                  v
CORE     deterministic transition -> fixed Store -> Event
                                  |
                                  v
HAL      immutable request -> driver binding -> hardware
```

| Layer | Owns |
| --- | --- |
| `agent-kernel-core` | Domain records, fixed Stores, deterministic transitions |
| `agent-kernel` | `no_std` syscall-style facade |
| `agent-kernel-x86_64` | Boot, privilege boundary, CPU frames, IRQ, faults |
| `agent-kernel-hal` | Immutable device request protocol |
| `agent-supervisor` | Host simulation and user-space orchestration |

## 03 / Agent Call ABI

```text
rax = magic       rbx = ABI version      rcx = operation / status
r8  = Agent       rdi = Task             rsi = Image
r9  = Nonce       r10..r15, rbp = bounded operation payload
```

| IDs | Protocol family |
| ---: | --- |
| `1-9` | Execution, verification, Mailbox IPC |
| `10-20` | Resource, Capability, Task, Agent lifecycle |
| `21-28` | Runtime Memory and Admission |
| `29-43` | Reclamation, compaction, Event archive |
| `44-50` | Namespace bind, resolve, mutation, compare, bounded path |

```text
userspace pointers : 0
identity source    : scheduled CPU context
reply shape        : canonical register frame
failure rule       : decode/auth/preflight before mutation
```

<details>
<summary><strong>ABI invariants</strong></summary>

- Core rechecks Capability scope and operation bits.
- Transactions preflight capacity, references, and Event slots.
- Canonical replies clear unrelated registers.
- Capsule, CPU-frame, and transcript checks cover native execution.

</details>

## 04 / Namespace Protocol

| Call | ID | Authority | Transition |
| --- | ---: | --- | --- |
| `BindNamespaceEntry` | 44 | `Act` | Allocate monotonic Entry ID |
| `ResolveNamespaceEntry` | 45 | `Observe` | Return record and emit resolution evidence |
| `RebindNamespaceEntry` | 46 | `Act` | Replace object and advance revision |
| `RetireNamespaceEntry` | 47 | `Rollback` | Remove stable entry and return slot |
| `CompareAndRebindNamespaceEntry` | 48 | `Act` | Replace at expected revision |
| `CompareAndRetireNamespaceEntry` | 49 | `Rollback` | Retire at expected revision |
| `ResolveNamespacePath` | 50 | `Observe` per hop | Resolve one or two native segments |

```text
Workspace 1 -- Key 1 / Capability A --> Mount(Workspace 3)
Workspace 3 -- Key 2 / Capability B --> terminal Entry
```

## 05 / Runtime Matrix

| Subsystem | Native path | QEMU evidence |
| --- | --- | --- |
| Isolation | CR3 roots, GDT/TSS/IDT, ring transitions | `MULTI_AGENT_ISOLATION_OK` |
| Scheduling | FIFO dispatch, PIT preemption, CPU resume | `MULTI_AGENT_CONTEXT_SWITCH_OK` |
| Faults | `#UD`, `#GP`, `#PF`, route, repair, restart | `NATIVE_AGENT_FAULT_RESTART_OK` |
| IPC | Blocking Mailbox, wake, acknowledge, retire | `NATIVE_MAILBOX_IPC_OK` |
| Memory | Page/region allocation, First-Fit reuse, zeroing | `NATIVE_MEMORY_CONCURRENCY_OK` |
| Managers | Resource, Capability, Task, Agent, Memory, Namespace | `NATIVE_RESOURCE_MANAGER_AGENT_OK` |
| Admission | Resident Supervisor, permits, release batches | `NATIVE_RUNTIME_ADMISSION_COMMIT_OK` |
| Driver | UART IRQ, HAL request, native Invocation | `DRIVER_INVOCATION_FLOW_OK` |
| Audit | Event Log, SHA-256 archive, exact replay | `NATIVE_EVENT_ARCHIVE_REPLAY_OK` |

## 06 / Verified Profile

| Metric | Value |
| --- | ---: |
| Target | `x86_64-unknown-none` |
| Isolated Agent contexts | 11 |
| Kernel-selected dispatches | 35 |
| Resource Manager Calls / CR3 switches | `38 / 76` |
| Admission Supervisor Calls / CR3 switches | `44 / 88` |
| Namespace capacity / final occupancy | `2 / 1` |
| Live Event capacity / peak | `362 / 362` |
| Archived Events | 64 |
| Final live Events / next sequence | `332 / 397` |
| Complete transcript | Events `1..396` |

| Native Capsule | Calls | Bytes | SHA-256 |
| --- | ---: | ---: | --- |
| Resource Manager | 38 | 3,789 | `24d6a22464c9...08bdcc1a` |
| Admission Supervisor | 44 | 4,115 | `f6c4efffe3c5...6f72f3f2` |

<details>
<summary><strong>Digests / terminal event window</strong></summary>

```text
resource_manager
24d6a22464c9b2cc27826c6b07a4655a5510968286eaff7c632732b408bdcc1a

admission_supervisor
f6c4efffe3c58689f8cb926399dc3fcb675e938d95bba463130495696f72f3f2

event[185] namespace_entry_bound       # root mount
event[186] namespace_entry_bound       # child terminal
event[187] namespace_entry_resolved    # root hop
event[188] namespace_entry_resolved    # child hop
event[189] namespace_entry_rebound     # terminal revision 2
event[190] namespace_entry_retired     # root mount
...
event[396] driver_invocation_completed
SUPERVISOR_HANDOFF_READY
```

</details>

## 07 / Build + Boot

```bash
git clone https://github.com/Evan-master/agent-kernel.git
cd agent-kernel

cargo test --workspace
cargo run -p agent-supervisor
```

```bash
# Bare-metal transcript gates
scripts/run-qemu.sh
scripts/run-qemu.sh --release
```

```bash
# Bare-metal compile gate
cargo check \
  -p agent-kernel-x86_64 \
  --features bare-metal \
  --bin agent-kernel-x86_64 \
  --target x86_64-unknown-none
```

Toolchain: `rustup`, pinned nightly, LLVM tools, `x86_64-unknown-none`, QEMU.

## 08 / Repository Map

```text
crates/
|- agent-kernel-core/    deterministic no_std model + Stores
|- agent-kernel/         no_std syscall-style facade
|- agent-kernel-hal/     immutable device request protocol
|- agent-kernel-boot/    bootstrap + capacity profile
|- agent-kernel-x86_64/  boot + isolation + IRQ + faults + Agent Calls
|- agent-kernel-image/   BIOS image builder
`- agent-supervisor/     host Supervisor + virtual device backend

docs/superpowers/
|- specs/                approved architecture records
`- plans/                milestone implementation plans
```

## 09 / Roadmap

| Track | Current | Next |
| --- | --- | --- |
| Namespace | Mounts and bounded traversal | Three/four-hop user-memory transport |
| Memory | Private tables, page/region reuse | Dynamic page-table growth |
| Scheduling | Single-core FIFO and PIT | SMP, synchronization, TLB shootdown |
| Durability | Bounded SHA-256 archive chain | Crash-consistent signed storage |
| Devices | UART and Port I/O | Storage, Network, Graphics, USB |
| Agent software | Fixed-width Capsule | Package format and production loader |
| Assurance | Tests, QEMU transcript, ELF audit | Hardening and formal verification |

Latest design record:
[`Native Namespace Hierarchy V3`](docs/superpowers/specs/2026-07-20-native-namespace-hierarchy-v3-design.md)

## Contributing

- Read [`AGENTS.md`](AGENTS.md).
- Start runtime behavior with a failing test.
- Preserve explicit authority and deterministic Events.
- Attach focused tests and relevant QEMU proof.

## License

[`MIT`](LICENSE) / Copyright 2026 Ran
