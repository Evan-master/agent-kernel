<div align="center">

# `AGENT KERNEL`

`AUTHORITY // ISOLATION // DETERMINISTIC EVIDENCE`

**English** / [简体中文](README.zh-CN.md)

<p>
  <img alt="Rust nightly" src="https://img.shields.io/badge/Rust-nightly-111111?logo=rust&amp;logoColor=white">
  <img alt="no_std" src="https://img.shields.io/badge/core-no__std-238636">
  <img alt="x86_64" src="https://img.shields.io/badge/target-x86__64--unknown--none-0969da">
  <img alt="QEMU" src="https://img.shields.io/badge/QEMU-events_1..409-f97316">
  <img alt="MIT" src="https://img.shields.io/badge/license-MIT-d0d7de">
</p>

</div>

```console
$ scripts/run-qemu.sh --release

[boot]       AGENT_KERNEL_QEMU_BOOT_OK
[isolation]  AGENT_KERNEL_MULTI_AGENT_ISOLATION_OK
[agents]     AGENT_KERNEL_HETEROGENEOUS_AGENT_EXECUTION_OK
[namespace]  AGENT_KERNEL_AGENT_CALL_NAMESPACE_MEMORY_PATH_OK
[mutation]   AGENT_KERNEL_AGENT_CALL_NAMESPACE_TYPED_REBIND_OK
[audit]      AGENT_KERNEL_NATIVE_EVENT_ARCHIVE_REPLAY_OK
[event:409]  driver_invocation_completed
[handoff]    SUPERVISOR_HANDOFF_READY
```

```text
+------------------------ BUILD PROFILE -------------------------+
| STATUS    active development    | ABI       0.1 / unstable     |
| CORE      no_std / heap-free    | TARGET    x86_64 bare metal  |
| MACHINE   BIOS / ring 0 + 3     | PROOF     test/QEMU/ELF      |
+----------------------------------------------------------------+
```

<p align="center">
  <a href="#01--native-model"><code>MODEL</code></a> /
  <a href="#02--machine-path"><code>MACHINE</code></a> /
  <a href="#03--agent-call-abi"><code>ABI</code></a> /
  <a href="#04--typed-namespace"><code>NAMESPACE</code></a> /
  <a href="#06--verified-profile"><code>PROOF</code></a> /
  <a href="#07--build--boot"><code>BOOT</code></a> /
  <a href="#09--roadmap"><code>ROADMAP</code></a>
</p>

## 00 // Kernel Contract

| Signal | Enforced contract |
| :--- | :--- |
| `IDENTITY` | Call frame binds the scheduled Agent, Task, Image, and Nonce |
| `AUTHORITY` | Resource operations present an explicit Capability |
| `MUTATION` | Successful transitions emit ordered Events |
| `RECOVERY` | Checkpoint, Rollback, fault route, repair, restart |
| `ISOLATION` | Per-Agent CR3 root and ring-3 execution context |
| `I/O` | Authorized HAL request reaches native IRQ and port paths |

## 01 // Native Model

```text
 AGENT ---- presents ----> CAPABILITY ---- controls ----> RESOURCE
   |                                                          |
   +-------------------- emits EVENT <-------------------------+
```

```text
IDENTITY   Agent / Task / Image / Execution Context
AUTHORITY  Capability / Scope / Operation / Delegation
WORK       Intent / Action / Observation / Verification
RECOVERY   Checkpoint / Rollback / Fault / Restart
STRUCTURE  Workspace / Namespace / Entry / Revision
EVIDENCE   Event / Archive Digest / Replay
```

| Primitive | Kernel role |
| :--- | :--- |
| `Agent` | Authenticated authority subject with schedulable state |
| `Capability` | Derivable and revocable operations over one Resource |
| `Intent` | Typed declaration of desired work |
| `Task` | Schedulable work bound to delegated authority |
| `Verification` | Independent trust transition after execution |
| `Checkpoint` | Recovery point governed by Rollback authority |
| `Event` | Deterministic evidence for successful mutation |
| `Namespace` | Revisioned binding, Workspace Mount, bounded path |

```text
USER SPACE  model runtime | prompts | planning | external adapters
----------- int 0x90 / IRQ / Fault --------------------------------
KERNEL      identity | authority | scheduling | isolation | audit
```

## 02 // Machine Path

```text
RING 3    verified Capsule -> Agent -> int 0x90 / IRQ / Fault
                                      |
---------------- privilege boundary --|------------------------
                                      v
RING 0    x86_64 entry -> ABI decode -> authenticate -> facade
                                      |
                                      v
CORE      deterministic transition -> fixed Store -> Event
                                      |
                                      v
HAL       immutable request -> driver binding -> hardware
```

| Crate | Owns |
| :--- | :--- |
| `agent-kernel-core` | Domain records, fixed Stores, deterministic transitions |
| `agent-kernel` | `no_std` syscall-style facade |
| `agent-kernel-x86_64` | Boot, privilege boundary, CPU frames, IRQ, faults |
| `agent-kernel-hal` | Immutable device request protocol |
| `agent-supervisor` | Host simulation and user-space orchestration |

## 03 // Agent Call ABI

```text
+---------------------------- CALL FRAME -------------------------+
| rax  magic       | rbx  ABI version | rcx  operation / status  |
| r8   Agent       | rdi  Task        | rsi  Image               |
| r9   Nonce       | r10..r15, rbp    | bounded payload          |
+-----------------------------------------------------------------+
```

| IDs | Protocol family |
| ---: | :--- |
| `1-9` | Execution, verification, Mailbox IPC |
| `10-20` | Resource, Capability, Task, Agent lifecycle |
| `21-28` | Runtime Memory and Admission |
| `29-43` | Reclamation, compaction, Event archive |
| `44-52` | Namespace bind, resolve, compare, mutation, bounded paths |

```text
TRANSPORT  fixed private call-data page + typed records
POINTERS   arbitrary userspace pointers rejected
IDENTITY   derived from the scheduled CPU context
REPLY      canonical register frame
ORDER      decode -> authenticate -> preflight -> mutate
```

<details>
<summary><code>ABI_INVARIANTS</code></summary>

| Gate | Check |
| :--- | :--- |
| Core | Capability scope and operation bits |
| Transaction | Capacity, live references, Event slots |
| Reply | Unrelated registers cleared |
| Native proof | Capsule, CPU frame, transcript |

</details>

## 04 // Typed Namespace

| Call | ID | Authority | Transition |
| :--- | ---: | :--- | :--- |
| `BindNamespaceEntry` | 44 | `Act` | Allocate monotonic Entry ID |
| `ResolveNamespaceEntry` | 45 | `Observe` | Return record and resolution Event |
| `RebindNamespaceEntry` | 46 | `Act` | Replace object; advance revision |
| `RetireNamespaceEntry` | 47 | `Rollback` | Remove stable Entry; return slot |
| `CompareAndRebindNamespaceEntry` | 48 | `Act` | Replace at expected revision |
| `CompareAndRetireNamespaceEntry` | 49 | `Rollback` | Retire at expected revision |
| `ResolveNamespacePath` | 50 | Per-hop `Observe` | Resolve one or two native segments |
| `ResolveNamespacePathFromMemory` | 51 | Per-hop `Observe` | Snapshot and resolve three or four segments |
| `CompareAndRebindNamespacePathFromMemory` | 52 | Mount `Observe` + terminal `Act` | Atomic bounded-path compare and mutation |

```text
CALL_DATA[160]
+0x00  magic[8]       "AGNTMSG1"
+0x08  version[8]     1
+0x10  generation[8]
+0x18  kind[8]        COMPARE_AND_REBIND_NAMESPACE_PATH
+0x20  total_len[8]   160
+0x28  payload_len[8] 112
+0x30  segments[4]    NamespaceSegment[16]
+0x70  root[8]        WorkspaceId
+0x78  depth[8]       1..4
+0x80  expected_rev[8]
+0x88  replacement[8] NamespaceObject
+0x90  flags[8]       0
+0x98  reserved[8]    0
```

```text
Workspace 1 -- Key 1 / Cap A --> Mount(Workspace 3)
Workspace 3 -- Key 2 / Cap B --> Mount(Workspace 8)
Workspace 8 -- Key 3 / Cap C --> Mount(Workspace 9)
Workspace 9 -- Key 4 / Cap D --> Entry revision 1 -> 2 / Resource(3)

snapshot -> decode -> authenticate hops -> compare revision -> rebind
```

## 05 // Runtime Matrix

| Subsystem | Native path | QEMU evidence |
| :--- | :--- | :--- |
| Isolation | CR3 roots, GDT/TSS/IDT, ring transitions | `MULTI_AGENT_ISOLATION_OK` |
| Scheduling | FIFO dispatch, PIT preemption, CPU resume | `MULTI_AGENT_CONTEXT_SWITCH_OK` |
| Faults | `#UD`, `#GP`, `#PF`, route, repair, restart | `NATIVE_AGENT_FAULT_RESTART_OK` |
| IPC | Blocking Mailbox, wake, acknowledge, retire | `NATIVE_MAILBOX_IPC_OK` |
| Memory | Page/region allocation, First-Fit reuse, zeroing | `NATIVE_MEMORY_CONCURRENCY_OK` |
| Managers | Resource, Capability, Task, Agent, Memory, Namespace | `NATIVE_RESOURCE_MANAGER_AGENT_OK` |
| Admission | Resident Supervisor, permits, release batches | `NATIVE_RUNTIME_ADMISSION_COMMIT_OK` |
| Driver | UART IRQ, HAL request, native Invocation | `DRIVER_INVOCATION_FLOW_OK` |
| Audit | Event Log, SHA-256 archive, exact replay | `NATIVE_EVENT_ARCHIVE_REPLAY_OK` |

## 06 // Verified Profile

| Metric | Value |
| :--- | ---: |
| Target | `x86_64-unknown-none` |
| Isolated Agent contexts | 11 |
| Kernel-selected dispatches | 35 |
| Resource Manager Calls / CR3 switches | `43 / 86` |
| Admission Supervisor Calls / CR3 switches | `44 / 88` |
| Namespace capacity / final occupancy | `4 / 4` |
| Live Event capacity / peak | `375 / 375` |
| Archived Events | 64 |
| Final live Events / next sequence | `345 / 410` |
| Complete transcript | Events `1..409` |

| Native Capsule | Calls | Bytes | SHA-256 |
| :--- | ---: | ---: | :--- |
| Resource Manager | 43 | 4,125 | `0bd528c2ae19...11a12c50` |
| Admission Supervisor | 44 | 4,115 | `4abda1fd3040...f45662dd` |

<details>
<summary><code>RAW_PROOF</code></summary>

```text
resource_manager
0bd528c2ae19772d3810ba54018035ff98d72ef03666dfe5872f4c0211a12c50

admission_supervisor
4abda1fd30408ce5e24f1ce19dba523c04d3edc6bde2dc6ee014414ff45662dd

event[185]       namespace_entry_bound
event[186]       namespace_entry_bound
event[187..188]  namespace_entry_resolved
event[207]       namespace_entry_rebound
event[208..209]  namespace_entry_bound
event[210..213]  namespace_entry_resolved  / Call 51
event[214..216]  namespace_entry_resolved  / Call 52 mounts
event[217]       namespace_entry_rebound   / revision 2 / Resource(3)
...
event[409]       driver_invocation_completed
SUPERVISOR_HANDOFF_READY
```

</details>

## 07 // Build + Boot

```console
$ git clone https://github.com/Evan-master/agent-kernel.git
$ cd agent-kernel
$ cargo test --workspace
$ cargo run -p agent-supervisor
```

```console
$ scripts/run-qemu.sh
$ scripts/run-qemu.sh --release
```

```console
$ cargo check \
    -p agent-kernel-x86_64 \
    --features bare-metal \
    --bin agent-kernel-x86_64 \
    --target x86_64-unknown-none
```

| Toolchain component | Profile |
| :--- | :--- |
| Rust | Pinned nightly via `rust-toolchain.toml` |
| Target | `x86_64-unknown-none` |
| Image | BIOS boot image |
| Runtime | QEMU x86_64 |
| Binary audit | LLVM tools |

## 08 // Source Tree

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

## 09 // Roadmap

| Track | Current | Next |
| :--- | :--- | :--- |
| Namespace | Typed fixed-page messages; four-hop mutation | Bulk messages; delegated mutation |
| Memory | Private tables; page/region reuse | Dynamic page-table growth |
| Scheduling | Single-core FIFO; PIT | SMP; synchronization; TLB shootdown |
| Durability | Bounded SHA-256 archive chain | Crash-consistent signed storage |
| Devices | UART; Port I/O | Storage; Network; Graphics; USB |
| Agent software | Fixed-width Capsule | Package format; production loader |
| Assurance | Tests; QEMU transcript; ELF audit | Hardening; formal verification |

`CURRENT_SPEC` [`Typed Namespace Path Rebind V5`](docs/superpowers/specs/2026-07-21-typed-namespace-path-rebind-v5-design.md)

## 10 // Patch Protocol

| Gate | Requirement |
| :--- | :--- |
| `CONTRACT` | Read [`AGENTS.md`](AGENTS.md) |
| `RED` | Start runtime behavior with a failing test |
| `MODEL` | Preserve explicit authority and deterministic Events |
| `PROOF` | Attach focused tests and relevant QEMU evidence |

```text
LICENSE  MIT
COPYRIGHT 2026 Ran
```
