<div align="center">

# `AGENT KERNEL`

**A native ring-0 substrate for autonomous software.**

**English** / [简体中文](README.zh-CN.md)

<p>
  <img alt="Rust nightly" src="https://img.shields.io/badge/Rust-nightly-111111?logo=rust&amp;logoColor=white">
  <img alt="no_std" src="https://img.shields.io/badge/kernel-no__std-238636">
  <img alt="x86_64" src="https://img.shields.io/badge/arch-x86__64-0969da">
  <img alt="QEMU" src="https://img.shields.io/badge/proof-QEMU-f97316">
  <img alt="MIT" src="https://img.shields.io/badge/license-MIT-d0d7de">
</p>

<pre>
agent-kernel / native-x86_64
[00] identity ............... bound
[01] capability graph ....... online
[02] signer algorithms ...... verified
[03] ring-3 agents .......... isolated
[04] durable boot chain ..... armed
[05] native state signer .... TPM-bound
kernel://state-signer/v19-crb
</pre>

</div>

```text
┌─ SYSTEM STATUS ─────────────────────────────────────────────────┐
│ VERIFIED   V10 / QEMU debug + release   HEAD  V19 native TPM    │
│ KERNEL     no_std / heap-free           ISA    x86_64           │
│ MODE       ring 0 + ring 3              ABI    Agent Call       │
│ STATE      ATA LBA48 A/B slots          AUTH   Capabilities     │
└─────────────────────────────────────────────────────────────────┘
```

## `00 // SIGNAL`

```text
IDENTITY    Agent / Task / Image / ExecutionContext
AUTHORITY   Capability / Scope / Operation / Delegation
WORK        Intent / Action / Observation / Verification
RECOVERY    Checkpoint / Rollback / Fault / Restart
STRUCTURE   Workspace / Namespace / Entry / Revision
EVIDENCE    Event / ArchiveDigest / Replay
```

| Kernel rule | Result |
| :--- | :--- |
| Calls inherit identity | No caller-supplied Agent identity |
| Capabilities gate mutation | Authority stays explicit and revocable |
| State transitions emit Events | Execution stays replayable and auditable |
| Agents own address spaces | Native workloads cross a real privilege boundary |

## `01 // MACHINE`

```text
RING 3   signed package ──> Agent ──> int 0x90 / IRQ / Fault
                                       │
────────────── privilege boundary ─────┼─────────────────────────
                                       ▼
RING 0   x86_64 entry ──> ABI decode ──> authorize ──> facade
                                       │
                                       ▼
CORE     deterministic transition ──> fixed Store ──> Event
                                       │
                                       ▼
HAL      immutable request ──> driver binding ──> hardware
```

| Layer | Responsibility |
| :--- | :--- |
| `agent-kernel-core` | Records, fixed-capacity Stores, transitions, Events |
| `agent-kernel` | Stable `no_std` syscall-style facade |
| `agent-kernel-x86_64` | Boot, paging, ring transitions, IRQ, ATA PIO, TPM CRB, native execution |
| `agent-kernel-hal` | Immutable device-request protocol |
| `agent-state-signer` | `no_std` signing policy and injected provider boundary |
| `agent-supervisor` | Host simulation and user-space orchestration |

## `02 // EXECUTION`

```text
Agent package
    ├── identity digest
    ├── capability set
    ├── private CR3 root
    ├── RX code + R/NX rodata
    ├── guarded stack + lazy page
    ├── typed call-data page
    └── deterministic Event stream
```

| Subsystem | Native path |
| :--- | :--- |
| Scheduling | FIFO dispatch, per-CPU Local APIC quantum, CPU-frame resume |
| Isolation | Per-Agent page tables, GDT/TSS/IDT, ring-3 entry |
| Recovery | `#UD`, `#GP`, `#PF`, repair, restart, rollback |
| IPC | Blocking mailbox, wake, acknowledge, retire |
| Memory | Page/region allocation, first-fit reuse, zeroing |
| I/O | Capability-authorized HAL request, I/O APIC IRQ, port and ATA PIO access |

<details>
<summary><code>USER ADDRESS MAP</code></summary>

```text
0x4000_0000_0000..ffff  code / 16 pages      RX
0x4000_0001_0000..ffff  rodata / 16 pages    R + NX
0x4000_0002_0000        signal page          R + NX
0x4000_0002_1000        guard page           unmapped
0x4000_0002_2000..5fff  stack / 4 pages      RW + NX
0x4000_0002_6000        lazy page            on demand
0x4000_0002_7000..ffff  runtime / 9 pages    capability governed
0x4000_0003_0000        call-data page       typed records
```

</details>

## `03 // TRUST CHAIN`

```text
SHA-256 identity ──> canonical envelope ──> signer ID
                                                │
                                                ▼
Trust Policy ──> kind + ABI scope ──> Ed25519 verify_strict
                                                │
                                                ▼
exact frames ──> ABS64 relocation ──> RX / R+NX ──> ring 3
```

```text
AGNTIMG\0 / Package v3
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

| Bound | Contract |
| :--- | :--- |
| Envelope | Canonical offsets, lengths, counts, reserved fields |
| Relocations | `0..64`, sorted, disjoint, page-contained |
| Signature | Exact prefix before the final 64-byte signature |
| Admission | Active signer, matching image kind and ABI range |
| Mapping | Code `RX`, rodata `R+NX`, no writable executable page |

## `04 // DURABLE STATE`

```text
Event prefix ──> canonical payload ──> 285B manifest V1/V2
                                                │
                               ┌────────────────┴────────────────┐
                               ▼                                 ▼
                          Ed25519                         P-256/SHA-256
                               └────────────────┬────────────────┘
                                                ▼
slot A/B ──> Prepared + flush ──> body + flush ──> readback verify
                                                │
                                                ▼
Committed footer + flush ──> receipt ──> one-shot Core proof ──> release
```

```text
prepare(54) ──> private call-data ──> State Signer policy
                                              │
                     sign(56) ──> kernel TPM service ──> CRB
                                              │
                                              ▼
commit(55) <── exact 384B request <── low-S P-256 signature
```

| Contract | V13 through V19 invariant |
| :--- | :--- |
| Slot | `64 KiB`; odd generations use `A`, even generations use `B` |
| Payload | Exact Event Archive digest preimage; maximum `64 KiB - 512` |
| Manifest | V1 preserves legacy Ed25519 bytes; V2 binds an explicit algorithm |
| Signature | 64 bytes: strict Ed25519 or IEEE P1363 low-S ECDSA P-256/SHA-256 |
| Signer ID | Legacy Ed25519 domain retained; algorithm-bound keys use the V2 domain |
| Transaction | 8 explicit write, flush, and readback fault boundaries |
| Recovery | Highest connected signed head; split and disconnected heads fail closed |
| Boot import | Virgin Core only; next Event starts at `through_sequence + 1` |
| Signed request | Canonical 384-byte record; only signature bytes `317..381` may change |
| Signer Agent | First-class image and entry identity, independent policy, injected provider |
| Core gate | Raw receipts cannot release Events; verified commits are consumed once |
| Native device | ATA LBA48, 512-byte sectors, bounded polling, `FLUSH CACHE EXT` |
| Native mapping | 128 sectors per slot; one aligned 256-sector reserved range |
| TPM authority | Ring 0 owns MMIO and command transport; ring 3 can request one retained-manifest signature |

```text
ATA IDENTIFY ──> dual-slot scan ──> chain + signature verification
                                         │
                   ┌─────────────────────┴─────────────────────┐
                   ▼                                           ▼
              GENESIS BOOT                          RECOVERED(generation)
                   │                                           │
                   └────────> stable Resources <──── one-shot Core proof
```

```text
V17 NATIVE STATE SIGNER
entry.S + immutable policy + external provider.o
                 │
                 ▼
fixed x86_64 link ──> ELF section audit ──> Package v3 / kind 5
                 │
                 ▼
        external Ed25519 image signature
```

| Native signer boundary | Contract |
| :--- | :--- |
| Core identity | `AgentImageKind::StateSigner` + `AgentEntryKind::StateSigner` |
| Image trust | x86 kind `5`; independent signer scope bit `4` |
| Provider ABI | Manifest, signature, policy generation, authenticated Agent/Task/Image |
| Algorithm policy | Immutable Ed25519 or ECDSA P-256/SHA-256 selection |
| Package | Two fixed-address segments, zero relocations, output mode `0600` |
| Secret ownership | Provider retains durable-state key access; package contains public policy |

```text
V18 HARDWARE SIGNER AGILITY
manifest          V1 legacy Ed25519 | V2 algorithm-bound
public key        Ed25519 / 32B | compressed SEC1 P-256 / 33B
signature         Ed25519 / 64B | IEEE P1363 low-S P-256 / 64B
failure policy    mismatch / malformed key / high-S -> fail closed
```

```text
V19 NATIVE TPM STATE SIGNER
discovery         ACPI TPM2 / Start Method 7
transport         CRB locality 0 / bounded polls / fail-closed cleanup
binding           ReadPublic Name + template + compressed P-256 point
agent boundary    Call 56 / retained manifest only / no raw TPM channel
recovery proof    TPM sign / ATA commit / power loss / cold recovery
```

`ATA BACKEND` complete · `STATE SIGNER PACKAGE` complete · `TPM CRB PATH` complete

## `05 // AGENT CALL`

```text
┌─ REGISTER FRAME ────────────────────────────────────────────────┐
│ rax magic    rbx ABI       rcx operation / status              │
│ r8  Agent    rdi Task      rsi Image      r9 Nonce             │
│ r10..r15 + rbp             bounded payload                     │
└────────────────────────────────────────────────────────────────┘

decode → snapshot → authenticate → preflight → mutate → reply
```

| IDs | Protocol family |
| ---: | :--- |
| `1..9` | Execution, verification, mailbox IPC |
| `10..20` | Resource, capability, Task, Agent lifecycle |
| `21..28` | Runtime memory and admission |
| `29..43` | Reclamation, compaction, Event archive |
| `44..52` | Namespace bind, resolve, compare, mutation, paths |
| `53` | Agent image signer-policy rotation |
| `54..56` | Durable archive prepare, TPM sign, and signed commit |

`TRANSPORT` private call-data page · `POINTERS` rejected · `REPLY` canonical registers

## `06 // PROOF`

```text
PROFILE            V10 signed-v3
QEMU               debug + release
EVENTS             1..412 / exact replay
AGENT CONTEXTS      11 isolated
DISPATCHES          35
FRAME OWNERSHIP     12..43 per Agent
BOOT FRAME POOL     77 sealed
```

| Proof surface | Signal |
| :--- | :--- |
| Signed package | `AGENT_KERNEL_NATIVE_SIGNED_PACKAGE_OK` |
| Isolation | `AGENT_KERNEL_MULTI_AGENT_ISOLATION_OK` |
| Context switching | `AGENT_KERNEL_MULTI_AGENT_CONTEXT_SWITCH_OK` |
| Fault recovery | `AGENT_KERNEL_NATIVE_AGENT_FAULT_RESTART_OK` |
| Namespace paths | `AGENT_KERNEL_AGENT_CALL_NAMESPACE_MEMORY_PATH_OK` |
| Archive replay | `AGENT_KERNEL_NATIVE_EVENT_ARCHIVE_REPLAY_OK` |
| Handoff | `SUPERVISOR_HANDOFF_READY` |

```text
V13 HOST RECEIPT
slot=A  generation=1  flush_epoch=3
archive=b72f0e90513d...e823449aff0d
```

```text
V14 ATA CONTRACT
commit path       390 device operations
cold scan         256 sector reads
fault boundaries  body write / footer flush / committed readback
```

```text
V15 DURABLE BOOT
request record    384 canonical bytes
recovery import   one-shot / virgin Core / overflow checked
boot profile      Disabled | ATA
bare target       x86_64-unknown-none
```

```text
V16 STATE SIGNER
call IDs          54 prepare / 55 commit
signature window  bytes 317..381 only
session states    ready / prepared / faulted
closed loop       preflight / sign / ATA / release / cold recovery
```

```text
V17 FIRST-CLASS SIGNER
Core identity     StateSigner image + entry
trust scope       bit 4 / x86 image kind 5
native package    fixed address / 2 segments / 0 relocations
provider          external ABI / durable key excluded from package
```

```text
V18 SIGNER AGILITY
manifest          exact V1 compatibility / explicit V2 algorithm
verification      Ed25519 verify_strict / P-256 SHA-256 low-S
policy            provider + package + manifest must agree
closed loop       P-256 sign / ATA commit / power loss / cold recovery
```

```text
V19 NATIVE TPM
ACPI               checksum-valid TPM2 / Start Method 7
CRB                locality + ready + execute + cleanup
wire               ReadPublic / SignDigest v185 / Sign v184
key binding        Name + TPMT_PUBLIC + P-256 point
Agent Call         56 / generation-only payload
closed loop        TPM response / ATA commit / cold recovery
```

<details>
<summary><code>VERIFIED IMAGE INVENTORY</code></summary>

| Native image | Format | Calls | Bytes | SHA-256 |
| :--- | :--- | ---: | ---: | :--- |
| Resource Manager | Signed Package v3 | 43 | 16,738 | `8fed932cf0a4...6699f9b3d` |
| Admission Supervisor | Capsule v1 | 44 | 4,115 | `5a657ca1ecde...9339078` |

</details>

## `07 // BOOT`

```console
$ git clone https://github.com/Evan-master/agent-kernel.git
$ cd agent-kernel
$ cargo test --workspace
$ cargo run -p agent-supervisor
```

```console
$ scripts/run-qemu.sh
$ scripts/run-qemu.sh --release
$ scripts/audit-agent-images.rb --assembly
$ ruby scripts/test-state-signer-package.rb
```

```console
$ ruby scripts/build-state-signer-package.rb \
    --signature-algorithm ecdsa-p256-sha256 \
    --image-key "$IMAGE_KEY" --kernel-tpm-provider \
    --output "$STATE_SIGNER_PACKAGE" \
    --nonce 1 --archive-authority 2 --storage-authority 3 \
    --root 4 --storage 5 --through-sequence 64 \
    --call-data-generation 1 --policy-generation 1 \
    --state-signer-id "$STATE_SIGNER_ID"
```

```console
$ scripts/inspect-tpm-state-signer.rb \
    --handle 0x81010001 --command sign-digest-v185 \
    --policy-generation 1 \
    --name "$TPM_NAME" --public-key "$TPM_PUBLIC_KEY"
```

The hardware profile defaults to `Disabled`. Activation uses
`NativeTpmSignerProfile::Crb` and a matching ATA signer record.

```console
$ cargo check -p agent-kernel-x86_64 \
    --features bare-metal \
    --bin agent-kernel-x86_64 \
    --target x86_64-unknown-none
```

`TOOLCHAIN` Rust nightly · `EMULATOR` QEMU x86_64 · `TARGET` x86_64-unknown-none

## `08 // TREE`

```text
crates/
├─ agent-kernel-core/    deterministic kernel model
├─ agent-kernel/         no_std facade
├─ agent-kernel-hal/     hardware request protocol
├─ agent-kernel-boot/    bootstrap profile
├─ agent-kernel-x86_64/  native machine boundary
├─ agent-kernel-image/   BIOS image builder
├─ agent-state-signer/   no_std signing-policy Agent
└─ agent-supervisor/     host supervisor

docs/superpowers/{specs,plans}/
scripts/{run-qemu.sh,audit-agent-images.rb,build-state-signer-package.rb,inspect-tpm-state-signer.rb}
```

## `09 // ROADMAP`

```text
[done] Agent-native authority + deterministic Events
[done] ring-3 isolation + per-Agent address spaces
[done] typed Namespace + bounded path mutation
[done] Package v3 + Ed25519 boot trust
[done] runtime signer rotation + trust-policy Events
[done] signed durable state + dual-slot host recovery
[done] SMP + synchronization + TLB shootdown
[done] native ATA PIO adapter + signed cold recovery
[done] verified durable boot + Event sequence continuation
[done] State Signer Agent + native archive prepare/commit calls
[done] first-class signer identity + external-provider native package
[done] V1/V2 signer agility + low-S ECDSA P-256/SHA-256
[done] ACPI TPM2 discovery + CRB transport + provisioned signer binding
[done] Agent Call 56 + built-in TPM provider + scripted TPM recovery proof
[next] measured-boot policy sessions + sealed TPM authorization
[next] dedicated QEMU ATA image + emulator power-loss proof
[next] network + graphics + USB + formal verification
```

| Track | Record |
| :--- | :--- |
| Verified baseline | [Signed Agent Package V10](docs/superpowers/specs/2026-07-21-signed-agent-package-v10-design.md) |
| Runtime milestone | [SMP Runtime V12](docs/superpowers/specs/2026-07-23-smp-runtime-v12-design.md) |
| Durable protocol | [Signed Durable State V13](docs/superpowers/specs/2026-07-23-signed-durable-state-v13-design.md) |
| Native storage | [Native ATA Durable State V14](docs/superpowers/specs/2026-07-23-native-ata-durable-state-v14-design.md) |
| Active milestone | [Native TPM State Signer V19](docs/superpowers/specs/2026-07-24-native-tpm-state-signer-v19-design.md) |

## `10 // PROJECT`

| Field | Value |
| :--- | :--- |
| Engineering contract | [`AGENTS.md`](AGENTS.md) |
| License | [`MIT`](LICENSE) |
| Status | Active development |

```text
AGENT KERNEL // CONTROL PLANE FOR AUTONOMOUS MACHINES // 2026
```
