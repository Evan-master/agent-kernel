<div align="center">

# `AGENT KERNEL`

### `Native kernel substrate for autonomous agents.`

**English** / [简体中文](README.zh-CN.md)

<p>
  <img alt="Rust nightly" src="https://img.shields.io/badge/Rust-nightly-111111?logo=rust&amp;logoColor=white">
  <img alt="no_std" src="https://img.shields.io/badge/kernel-no__std-238636">
  <img alt="x86_64" src="https://img.shields.io/badge/arch-x86__64-0969da">
  <img alt="QEMU" src="https://img.shields.io/badge/proof-QEMU-f97316">
  <img alt="MIT" src="https://img.shields.io/badge/license-MIT-d0d7de">
</p>

<pre>
agent-kernel@ring0:~$ boot --profile signed-v3
[ OK ] immutable Trust Policy ....... loaded
[ OK ] strict Ed25519 signature ..... verified
[ OK ] isolated Agent address spaces online
[ OK ] segmented RX / R+NX image .... mapped
[ OK ] deterministic Event archive .. sealed
kernel://supervisor/handoff-ready
</pre>

</div>

```text
┌─ AGENT KERNEL // V10 ─────────────────────────────────────────┐
│ CORE      no_std / heap-free    TARGET    x86_64 bare metal   │
│ MODE      ring 0 + ring 3       FORMAT    Signed Package v3   │
│ AUTH      Capabilities          TRUST     Ed25519 policy      │
│ EVIDENCE  Tests / QEMU / ELF    STATUS    Active development  │
└───────────────────────────────────────────────────────────────┘
```

`MODEL` · `MACHINE` · `PACKAGE` · `ABI` · `EVIDENCE` · `BOOT`

## `00 / KERNEL SIGNAL`

| Channel | Definition |
| :--- | :--- |
| `SUBJECT` | Agent identity bound to Task, Image, and execution nonce |
| `AUTHORITY` | Explicit, derivable, revocable Capabilities |
| `WORK` | Intent → Action → Observation → Verification |
| `STATE` | Resource, Namespace, Checkpoint, Rollback |
| `EVIDENCE` | Ordered Events, archive digest, exact replay |

```text
AGENT ──presents──> CAPABILITY ──controls──> RESOURCE
  │                                                │
  └──────────────── emits EVENT <─────────────────┘
```

## `01 / NATIVE MODEL`

| Kernel invariant | Enforced contract |
| :--- | :--- |
| `IDENTITY` | Calls inherit the currently scheduled Agent context |
| `AUTHORITY` | Protected transitions require matching Capabilities |
| `MUTATION` | Successful state changes append deterministic Events |
| `ISOLATION` | Every native Agent owns a CR3 root and ring-3 frame |
| `RECOVERY` | Fault routing, repair, restart, Checkpoint, Rollback |
| `I/O` | Authorized HAL requests reach native IRQ and port paths |

```text
IDENTITY    Agent / Task / Image / ExecutionContext
AUTHORITY   Capability / Scope / Operation / Delegation
WORK        Intent / Action / Observation / Verification
RECOVERY    Checkpoint / Rollback / Fault / Restart
STRUCTURE   Workspace / Namespace / Entry / Revision
EVIDENCE    Event / ArchiveDigest / Replay
```

## `02 / MACHINE PATH`

```text
RING 3   verified Package ──> Agent ──> int 0x90 / IRQ / Fault
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

| Crate | Boundary |
| :--- | :--- |
| `agent-kernel-core` | Domain records, fixed Stores, deterministic transitions |
| `agent-kernel` | `no_std` syscall-style facade |
| `agent-kernel-x86_64` | Boot, CPU frames, isolation, IRQ, faults |
| `agent-kernel-hal` | Immutable device-request protocol |
| `agent-supervisor` | Host simulation and user-space orchestration |

## `03 / SIGNED PACKAGE V3`

```text
AGNTIMG\0 // Package v3
┌──────────────┬───────────────────┬──────────────────┐
│ header / 88B │ 2 segment records │ ABS64 records    │
├──────────────┴───────────────────┴──────────────────┤
│ code / 1..64 KiB / R+X                             │
├─────────────────────────────────────────────────────┤
│ rodata / 1..64 KiB / R+NX                          │
├─────────────────────────────────────────────────────┤
│ Ed25519 signature / 64B                            │
└─────────────────────────────────────────────────────┘
```

```text
SHA-256 identity → canonical envelope → signer ID
                                          │
                                          ▼
immutable Trust Policy → kind + ABI scope → Ed25519 verify_strict
                                          │
                                          ▼
exact frames → ABS64 patch → code RX + rodata R+NX → ring 3
```

| Contract | V10 bound |
| :--- | :--- |
| `ENVELOPE` | Canonical offsets, lengths, counts, and reserved fields |
| `SEGMENTS` | Exactly two: code, rodata |
| `RELOCATIONS` | `0..64`, sorted, non-overlapping, page-contained |
| `SIGNED MESSAGE` | Exact package prefix before the 64-byte signature |
| `SIGNER ID` | `SHA-256(domain || public_key)` |
| `TRUST` | One active signer entry with matching kind and ABI scope |
| `SIGNATURE` | Strict Ed25519 verification |
| `DIGEST` | Binds the complete package, including the signature |
| `LEGACY` | V1/V2 remain digest-pinned during inventory migration |

## `04 / USER MAP`

```text
0x4000_0000_0000..ffff  code / 16 pages      RX
0x4000_0001_0000..ffff  rodata / 16 pages    R + NX
0x4000_0002_0000        signal page          R + NX
0x4000_0002_1000        guard page           unmapped
0x4000_0002_2000..5fff  stack / 4 pages      RW + NX
0x4000_0002_6000        lazy page            on demand
0x4000_0002_7000        runtime page         capability governed
0x4000_0002_8000..ffff  runtime / 8 pages    capability governed
0x4000_0003_0000        call-data page       typed fixed records
```

```text
FRAME IDENTITY
page tables   4     code      1..16     rodata    0..16
signal        1     stack         4     lazy          1
call data     1     owned     12..43     pool         77
```

## `05 / AGENT CALL ABI`

```text
┌─ REGISTER FRAME ────────────────────────────────────────────────┐
│ rax magic    rbx ABI       rcx operation / status              │
│ r8  Agent    rdi Task      rsi Image      r9 Nonce             │
│ r10..r15 + rbp             bounded payload                     │
└────────────────────────────────────────────────────────────────┘
```

| Call IDs | Protocol family |
| ---: | :--- |
| `1-9` | Execution, Verification, Mailbox IPC |
| `10-20` | Resource, Capability, Task, Agent lifecycle |
| `21-28` | Runtime Memory and Admission |
| `29-43` | Reclamation, compaction, Event archive |
| `44-52` | Namespace bind, resolve, compare, mutation, paths |

```text
TRANSPORT  private call-data page + typed records
POINTERS   arbitrary userspace pointers rejected
IDENTITY   derived from the scheduled CPU context
REPLY      canonical register frame
ORDER      decode → authenticate → preflight → mutate
```

<details>
<summary><code>NAMESPACE // CALLS 44..52</code></summary>

```text
Workspace 1 --Cap A--> Mount(3) --Cap B--> Mount(8)
Workspace 8 --Cap C--> Mount(9) --Cap D--> Resource(3)

snapshot → decode → authenticate hops → compare → rebind
```

| Path | Contract |
| :--- | :--- |
| Bind / resolve | Stable Entry IDs and ordered resolution Events |
| Compare / mutate | Expected-revision guard and atomic transition |
| Bounded path | One to four segments with per-hop authority |
| Memory transport | Kernel snapshot before decode and validation |

</details>

## `06 / NATIVE MATRIX`

| Subsystem | Native mechanism | QEMU signal |
| :--- | :--- | :--- |
| Package | v3 parser, full digest, ABS64 relocation | `NATIVE_SIGNED_PACKAGE_OK` |
| Trust | Immutable signer policy, kind and ABI scope | `NATIVE_TRUSTED_SIGNER_OK` |
| Isolation | CR3, GDT/TSS/IDT, ring transitions | `MULTI_AGENT_ISOLATION_OK` |
| Scheduling | FIFO dispatch, PIT preemption, CPU resume | `MULTI_AGENT_CONTEXT_SWITCH_OK` |
| Faults | `#UD`, `#GP`, `#PF`, repair, restart | `NATIVE_AGENT_FAULT_RESTART_OK` |
| IPC | Blocking Mailbox, wake, acknowledge, retire | `NATIVE_MAILBOX_IPC_OK` |
| Memory | Page/region allocation, First-Fit reuse, zeroing | `NATIVE_MEMORY_CONCURRENCY_OK` |
| Managers | Resource, Task, Agent, Memory, Namespace | `NATIVE_RESOURCE_MANAGER_AGENT_OK` |
| Admission | Resident Supervisor, permits, batch release | `NATIVE_RUNTIME_ADMISSION_COMMIT_OK` |
| Driver | UART IRQ, HAL request, native Invocation | `DRIVER_INVOCATION_FLOW_OK` |
| Audit | SHA-256 archive chain and exact replay | `NATIVE_EVENT_ARCHIVE_REPLAY_OK` |

## `07 / VERIFIED PROFILE`

```text
QEMU TRANSCRIPT   Events 1..409      DISPATCH          35
AGENT CONTEXTS    11 isolated       NEXT SEQUENCE     410
CODE WINDOW       16 pages / 64 KiB RODATA WINDOW     16 pages / 64 KiB
FRAMES PER AGENT  12..43            BOOT FRAME POOL   77 sealed
EVENT STORE       375 peak / 345 final / 64 archived
```

| Native image | Format | Calls | Image bytes | SHA-256 |
| :--- | :--- | ---: | ---: | :--- |
| Resource Manager | Signed Package v3 | 43 | 16,738 | `8fed932cf0a4...6699f9b3d` |
| Admission Supervisor | Capsule v1 | 44 | 4,115 | `5a657ca1ecde...9339078` |

<details>
<summary><code>OPEN RAW BOOT PROOF</code></summary>

```console
$ scripts/run-qemu.sh --release

[boot]       AGENT_KERNEL_QEMU_BOOT_OK
[package]    AGENT_KERNEL_NATIVE_SEGMENTED_PACKAGE_OK
[signature]  AGENT_KERNEL_NATIVE_SIGNED_PACKAGE_OK
[trust]      AGENT_KERNEL_NATIVE_TRUSTED_SIGNER_OK
[rodata]     AGENT_KERNEL_NATIVE_RODATA_NX_OK
[relocation] AGENT_KERNEL_NATIVE_RELOCATION_OK
[isolation]  AGENT_KERNEL_MULTI_AGENT_ISOLATION_OK
[agents]     AGENT_KERNEL_HETEROGENEOUS_AGENT_EXECUTION_OK
[namespace]  AGENT_KERNEL_AGENT_CALL_NAMESPACE_MEMORY_PATH_OK
[audit]      AGENT_KERNEL_NATIVE_EVENT_ARCHIVE_REPLAY_OK
[event:409]  driver_invocation_completed
[handoff]    SUPERVISOR_HANDOFF_READY
```

</details>

## `08 / BUILD + BOOT`

```console
$ git clone https://github.com/Evan-master/agent-kernel.git
$ cd agent-kernel
$ cargo test --workspace
$ cargo run -p agent-supervisor
$ scripts/audit-agent-images.rb --assembly
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

## `09 / SOURCE MAP`

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

scripts/
├─ build-signed-resource-manager.rb  external-key v3 builder
├─ audit-agent-images.rb             package + ELF verifier
└─ run-qemu.sh                       debug / release boot proof
```

## `10 / ROADMAP`

```text
[x] Agent-native authority model
[x] ring-3 Agents + per-Agent address spaces
[x] deterministic Events + archive replay
[x] typed Namespace + bounded path mutation
[x] 64 KiB code windows + exact frame ownership
[x] Package v2 + RX/R+NX segments + ABS64 relocation
[x] Package v3 + Ed25519 signatures + boot Trust Policy
[>] signer rotation + runtime trust-policy Events
[ ] SMP + synchronization + TLB shootdown
[ ] storage / network / graphics / USB
[ ] signed durable state + formal verification
```

`CURRENT SPEC` · [`Signed Agent Package V10`](docs/superpowers/specs/2026-07-21-signed-agent-package-v10-design.md)

## `11 / ENGINEERING GATE`

| Gate | Requirement |
| :--- | :--- |
| `CONTRACT` | Follow [`AGENTS.md`](AGENTS.md) |
| `RED` | Begin runtime behavior with a failing contract test |
| `MODEL` | Preserve explicit authority and deterministic Events |
| `PROOF` | Pass focused tests, QEMU transcript, and ELF audit |

```text
AGENT KERNEL // MIT // COPYRIGHT 2026 RAN
```
