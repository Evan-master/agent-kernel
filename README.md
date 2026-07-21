<div align="center">

# `AGENT KERNEL`

### `A native operating substrate for autonomous agents.`

**English** / [简体中文](README.zh-CN.md)

<p>
  <img alt="Rust nightly" src="https://img.shields.io/badge/Rust-nightly-111111?logo=rust&amp;logoColor=white">
  <img alt="no_std" src="https://img.shields.io/badge/kernel-no__std-238636">
  <img alt="x86_64" src="https://img.shields.io/badge/arch-x86__64-0969da">
  <img alt="QEMU" src="https://img.shields.io/badge/proof-QEMU-f97316">
  <img alt="MIT" src="https://img.shields.io/badge/license-MIT-d0d7de">
</p>

<pre>
agent-kernel@bare-metal:~$ boot --profile native
[ OK ] capability authority online
[ OK ] per-agent address spaces online
[ OK ] 64 KiB agent code window online
[ OK ] right-sized code-frame ownership online
[ OK ] deterministic event chain online
kernel://supervisor/handoff-ready
</pre>

</div>

```text
┌─ SYSTEM // AGENT-KERNEL ───────────────────────────────────────┐
│ MODEL     agent-first       CORE      no_std / heap-free       │
│ TARGET    x86_64 bare metal ABI       0.1 / unstable           │
│ MACHINE   BIOS / ring 0+3   EVIDENCE  tests / QEMU / ELF       │
│ STATUS    active development LICENSE  MIT                      │
└────────────────────────────────────────────────────────────────┘
```

`MODEL` · `MACHINE` · `CAPSULE` · `ABI` · `EVIDENCE` · `BOOT`

## `00 / KERNEL SIGNAL`

Agent Kernel is an original operating-system kernel organized around agents,
capabilities, resources, verifiable work, recovery, and deterministic evidence.

```text
AGENT ──presents──> CAPABILITY ──controls──> RESOURCE
  │                                                │
  └──────────────── emits EVENT <─────────────────┘
```

| Contract | Kernel invariant |
| :--- | :--- |
| `IDENTITY` | Every call binds Agent, Task, Image, and Nonce |
| `AUTHORITY` | Every protected operation requires a Capability |
| `MUTATION` | Every successful transition emits an ordered Event |
| `ISOLATION` | Every native Agent owns a CR3 root and ring-3 context |
| `RECOVERY` | Checkpoint, Rollback, fault routing, repair, restart |
| `I/O` | Authorized HAL requests reach native IRQ and port paths |

## `01 / NATIVE MODEL`

```text
IDENTITY    Agent / Task / Image / ExecutionContext
AUTHORITY   Capability / Scope / Operation / Delegation
WORK        Intent / Action / Observation / Verification
RECOVERY    Checkpoint / Rollback / Fault / Restart
STRUCTURE   Workspace / Namespace / Entry / Revision
EVIDENCE    Event / ArchiveDigest / Replay
```

| Primitive | Responsibility |
| :--- | :--- |
| `Agent` | Authenticated authority subject with schedulable state |
| `Capability` | Derivable and revocable access to one Resource |
| `Intent` | Typed declaration of desired work |
| `Task` | Schedulable work bound to delegated authority |
| `Verification` | Independent trust transition after execution |
| `Checkpoint` | Recovery point governed by Rollback authority |
| `Event` | Deterministic evidence for successful mutation |
| `Namespace` | Revisioned bindings, Workspace mounts, bounded paths |

## `02 / MACHINE PATH`

```text
RING 3   verified Capsule ──> Agent ──> int 0x90 / IRQ / Fault
                                         │
──────────────── privilege boundary ─────┼──────────────────────
                                         ▼
RING 0   x86_64 entry ──> ABI decode ──> authenticate ──> facade
                                         │
                                         ▼
CORE     deterministic transition ──> fixed Store ──> Event
                                         │
                                         ▼
HAL      immutable request ──> driver binding ──> hardware
```

| Layer | Ownership |
| :--- | :--- |
| `agent-kernel-core` | Domain records, fixed Stores, deterministic transitions |
| `agent-kernel` | `no_std` syscall-style facade |
| `agent-kernel-x86_64` | Boot, CPU frames, isolation, IRQ, faults |
| `agent-kernel-hal` | Immutable device-request protocol |
| `agent-supervisor` | Host simulation and user-space orchestration |

## `03 / AGENT CAPSULE`

```text
Capsule v1
┌──────────────┬──────────────┬──────────────────────────────────┐
│ magic / ABI  │ length / SHA │ fixed-layout x86_64 code         │
└──────────────┴──────────────┴──────────────────────────────────┘
        verify ──> allocate ──> map RX ──> enter ring 3
```

```text
USER MAP
0x4000_0000_0000..ffff  code / 16 pages      RX
0x4000_0001_0000        signal page          R + NX
0x4000_0001_1000        guard page           unmapped
0x4000_0001_2000..5fff  stack / 4 pages      RW + NX
0x4000_0001_6000        lazy page            on demand
0x4000_0001_7000        runtime page         capability governed
0x4000_0001_8000..ffff  runtime / 8 pages    capability governed
0x4000_0002_0000        call-data page       typed fixed records
```

```text
V8 PROFILE
CODE WINDOW       64 KiB / 16-page bounded RX window
PHYSICAL IDENTITY 12..27 frames / exact Capsule size
CROSS-PAGE PROOF  Resource Manager completes from page 5
POOL INVENTORY    76 sealed frames / atomic resize + reuse
```

## `04 / AGENT CALL ABI`

```text
┌─ REGISTER FRAME ────────────────────────────────────────────────┐
│ rax magic    rbx ABI       rcx operation / status              │
│ r8  Agent    rdi Task      rsi Image      r9 Nonce             │
│ r10..r15 + rbp             bounded payload                     │
└────────────────────────────────────────────────────────────────┘
```

| Call IDs | Protocol family |
| ---: | :--- |
| `1-9` | Execution, verification, Mailbox IPC |
| `10-20` | Resource, Capability, Task, Agent lifecycle |
| `21-28` | Runtime Memory and Admission |
| `29-43` | Reclamation, compaction, Event archive |
| `44-52` | Namespace bind, resolve, compare, mutation, paths |

```text
TRANSPORT  private call-data page + typed records
POINTERS   arbitrary userspace pointers rejected
IDENTITY   derived from the scheduled CPU context
REPLY      canonical register frame
ORDER      decode -> authenticate -> preflight -> mutate
```

<details>
<summary><code>NAMESPACE // CALLS 44..52</code></summary>

| Path | Contract |
| :--- | :--- |
| Bind / resolve | Stable Entry IDs and ordered resolution Events |
| Compare / mutate | Expected-revision guard and atomic transition |
| Bounded path | One to four segments with per-hop authority |
| Memory transport | Kernel snapshot before decode and validation |

```text
Workspace 1 --Cap A--> Mount(3) --Cap B--> Mount(8)
Workspace 8 --Cap C--> Mount(9) --Cap D--> Resource(3)

snapshot -> decode -> authenticate hops -> compare -> rebind
```

</details>

## `05 / NATIVE MATRIX`

| Subsystem | Native mechanism | QEMU signal |
| :--- | :--- | :--- |
| Isolation | CR3, GDT/TSS/IDT, ring transitions | `MULTI_AGENT_ISOLATION_OK` |
| Scheduling | FIFO dispatch, PIT preemption, CPU resume | `MULTI_AGENT_CONTEXT_SWITCH_OK` |
| Faults | `#UD`, `#GP`, `#PF`, repair, restart | `NATIVE_AGENT_FAULT_RESTART_OK` |
| IPC | Blocking Mailbox, wake, acknowledge, retire | `NATIVE_MAILBOX_IPC_OK` |
| Memory | Page/region allocation, First-Fit reuse, zeroing | `NATIVE_MEMORY_CONCURRENCY_OK` |
| Managers | Resource, Task, Agent, Memory, Namespace | `NATIVE_RESOURCE_MANAGER_AGENT_OK` |
| Admission | Resident Supervisor, permits, batch release | `NATIVE_RUNTIME_ADMISSION_COMMIT_OK` |
| Driver | UART IRQ, HAL request, native Invocation | `DRIVER_INVOCATION_FLOW_OK` |
| Audit | SHA-256 archive chain and exact replay | `NATIVE_EVENT_ARCHIVE_REPLAY_OK` |

## `06 / VERIFIED PROFILE`

```text
QEMU TRANSCRIPT   Events 1..409
WORKSPACE TESTS   216 groups / 745 passed
DISPATCH          35 kernel-selected
AGENT CONTEXTS    11 isolated
CAPSULE WINDOW    16 pages / 64 KiB
FRAMES PER AGENT  12..27 / active code pages 1..16
BOOT FRAME POOL   76 sealed / zeroed on full return
EVENT STORE       375 peak / 345 final / 64 archived
NEXT SEQUENCE     410
```

| Native Capsule | Calls | Bytes | SHA-256 |
| :--- | ---: | ---: | :--- |
| Resource Manager | 43 | 16,480 | `3a8764b8c986...bdca8dc6e` |
| Admission Supervisor | 44 | 4,115 | `e09598b938db...c3bc04b01` |

<details>
<summary><code>OPEN RAW BOOT PROOF</code></summary>

```console
$ scripts/run-qemu.sh --release

[boot]       AGENT_KERNEL_QEMU_BOOT_OK
[isolation]  AGENT_KERNEL_MULTI_AGENT_ISOLATION_OK
[agents]     AGENT_KERNEL_HETEROGENEOUS_AGENT_EXECUTION_OK
[capsule]    AGENT_KERNEL_NATIVE_MULTI_PAGE_CAPSULE_OK
[capsule:5]  AGENT_KERNEL_NATIVE_FIFTH_CODE_PAGE_OK
[frames]     AGENT_KERNEL_NATIVE_RIGHT_SIZED_CODE_FRAMES_OK
[namespace]  AGENT_KERNEL_AGENT_CALL_NAMESPACE_MEMORY_PATH_OK
[mutation]   AGENT_KERNEL_AGENT_CALL_NAMESPACE_TYPED_REBIND_OK
[audit]      AGENT_KERNEL_NATIVE_EVENT_ARCHIVE_REPLAY_OK
[event:409]  driver_invocation_completed
[handoff]    SUPERVISOR_HANDOFF_READY
```

</details>

## `07 / BUILD + BOOT`

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
$ cargo check -p agent-kernel-x86_64 \
    --features bare-metal \
    --bin agent-kernel-x86_64 \
    --target x86_64-unknown-none
```

## `08 / SOURCE MAP`

```text
crates/
├─ agent-kernel-core/    deterministic model + fixed Stores
├─ agent-kernel/         no_std kernel facade
├─ agent-kernel-hal/     device-request protocol
├─ agent-kernel-boot/    bootstrap + capacity profile
├─ agent-kernel-x86_64/  machine boundary + native execution
├─ agent-kernel-image/   BIOS image builder
└─ agent-supervisor/     host Supervisor + virtual devices

docs/superpowers/
├─ specs/                architecture records
└─ plans/                milestone plans
```

## `09 / ROADMAP`

```text
[x] agent-native authority model
[x] ring-3 Capsules + per-Agent address spaces
[x] deterministic Events + archive replay
[x] typed Namespace + bounded path mutation
[x] sixteen-page Agent Capsules + fifth-page execution
[x] right-sized executable frame ownership
[>] segmented packages + relocations + signatures
[ ] SMP + synchronization + TLB shootdown
[ ] storage / network / graphics / USB
[ ] signed durable state + formal verification
```

`CURRENT SPEC` · [`Expanded Agent Capsule V8`](docs/superpowers/specs/2026-07-21-expanded-agent-capsule-v8-design.md)

## `10 / ENGINEERING GATE`

| Gate | Requirement |
| :--- | :--- |
| `CONTRACT` | Follow [`AGENTS.md`](AGENTS.md) |
| `RED` | Begin runtime behavior with a failing contract test |
| `MODEL` | Preserve explicit authority and deterministic Events |
| `PROOF` | Pass focused tests, QEMU transcript, and ELF audit |

```text
AGENT KERNEL // MIT // COPYRIGHT 2026 RAN
```
