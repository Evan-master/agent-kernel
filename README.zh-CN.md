<div align="center">

# `AGENT KERNEL`

**面向自主软件的原生 Ring 0 内核底座**

[English](README.md) / **简体中文**

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
[05] native state signer .... algorithm-bound
kernel://state-signer/v18-ready
</pre>

</div>

```text
┌─ SYSTEM STATUS ─────────────────────────────────────────────────┐
│ VERIFIED   V10 / QEMU debug + release   HEAD  V18 signer agility│
│ KERNEL     no_std / 无堆                 ISA    x86_64           │
│ MODE       ring 0 + ring 3              ABI    Agent Call       │
│ STATE      ATA LBA48 A/B slots          AUTH   Capability       │
└─────────────────────────────────────────────────────────────────┘
```

## `00 // 内核信号`

```text
IDENTITY    Agent / Task / Image / ExecutionContext
AUTHORITY   Capability / Scope / Operation / Delegation
WORK        Intent / Action / Observation / Verification
RECOVERY    Checkpoint / Rollback / Fault / Restart
STRUCTURE   Workspace / Namespace / Entry / Revision
EVIDENCE    Event / ArchiveDigest / Replay
```

| 内核规则 | 结果 |
| :--- | :--- |
| 调用继承身份 | Agent 身份无法由调用方自行填写 |
| Capability 控制修改 | 权限显式、可派生、可撤销 |
| 状态转换生成 Event | 执行过程支持重放与审计 |
| Agent 独占地址空间 | 原生工作负载穿越真实特权边界 |

## `01 // 机器路径`

```text
RING 3   signed package ──> Agent ──> int 0x90 / IRQ / Fault
                                       │
──────────────── 特权边界 ─────────────┼─────────────────────────
                                       ▼
RING 0   x86_64 入口 ──> ABI 解码 ──> 鉴权 ──> Facade
                                       │
                                       ▼
CORE     确定性转换 ──> 固定 Store ──> Event
                                       │
                                       ▼
HAL      不可变请求 ──> Driver Binding ──> Hardware
```

| 层 | 职责 |
| :--- | :--- |
| `agent-kernel-core` | 领域记录、固定容量 Store、状态转换、Event |
| `agent-kernel` | 稳定的 `no_std` syscall 风格 Facade |
| `agent-kernel-x86_64` | 启动、分页、特权切换、IRQ、ATA PIO、原生执行 |
| `agent-kernel-hal` | 不可变设备请求协议 |
| `agent-state-signer` | `no_std` 签名策略与可注入 Provider 边界 |
| `agent-supervisor` | 宿主模拟与用户空间编排 |

## `02 // 执行单元`

```text
Agent Package
    ├── identity digest
    ├── capability set
    ├── private CR3 root
    ├── RX code + R/NX rodata
    ├── guarded stack + lazy page
    ├── typed call-data page
    └── deterministic Event stream
```

| 子系统 | 原生路径 |
| :--- | :--- |
| 调度 | FIFO 派发、每 CPU Local APIC 量子、CPU Frame 恢复 |
| 隔离 | 每 Agent 页表、GDT/TSS/IDT、ring-3 入口 |
| 恢复 | `#UD`、`#GP`、`#PF`、修复、重启、回滚 |
| IPC | 阻塞 Mailbox、唤醒、确认、回收 |
| 内存 | 页/区域分配、First-Fit 复用、清零 |
| I/O | Capability 授权的 HAL 请求、I/O APIC IRQ、端口与 ATA PIO 访问 |

<details>
<summary><code>用户地址空间</code></summary>

```text
0x4000_0000_0000..ffff  code / 16 页        RX
0x4000_0001_0000..ffff  rodata / 16 页      R + NX
0x4000_0002_0000        signal page         R + NX
0x4000_0002_1000        guard page           未映射
0x4000_0002_2000..5fff  stack / 4 页        RW + NX
0x4000_0002_6000        lazy page           按需映射
0x4000_0002_7000..ffff  runtime / 9 页      Capability 治理
0x4000_0003_0000        call-data page      类型化记录
```

</details>

## `03 // 信任链`

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

| 边界 | 契约 |
| :--- | :--- |
| Envelope | 规范 offset、length、count、reserved 字段 |
| Relocation | `0..64`，有序、无重叠、目标不跨页 |
| Signature | 最后 64 字节签名前的精确 Package 前缀 |
| Admission | Active signer、匹配的镜像 kind 与 ABI 范围 |
| Mapping | code `RX`、rodata `R+NX`、无可写可执行页 |

## `04 // 持久状态`

```text
Event 前缀 ──> canonical payload ──> 285B manifest V1/V2
                                                │
                               ┌────────────────┴────────────────┐
                               ▼                                 ▼
                          Ed25519                         P-256/SHA-256
                               └────────────────┬────────────────┘
                                                ▼
slot A/B ──> Prepared + flush ──> body + flush ──> readback verify
                                                │
                                                ▼
Committed footer + flush ──> receipt ──> 一次性 Core proof ──> release
```

```text
prepare(54) ──> 私有 call-data ──> State Signer policy
                                            │
                                            ▼
commit(55) <── 精确 384B request <── provider signature
```

| 契约 | V13 至 V18 不变量 |
| :--- | :--- |
| 槽位 | `64 KiB`；奇数 generation 使用 `A`，偶数 generation 使用 `B` |
| Payload | Event Archive 摘要的精确原像；上限 `64 KiB - 512` |
| Manifest | V1 保留 Ed25519 历史字节；V2 显式绑定算法 |
| Signature | 固定 64 字节：严格 Ed25519 或 IEEE P1363 低 S ECDSA P-256/SHA-256 |
| Signer ID | Ed25519 历史域保持稳定；算法绑定密钥使用 V2 域 |
| Transaction | 8 个显式 write、flush、readback 故障边界 |
| Recovery | 选择最高的连续签名链头；分叉与断链均关闭自动恢复 |
| Boot Import | 仅允许空白 Core；下一条 Event 从 `through_sequence + 1` 开始 |
| Signed Request | 384 字节 canonical 记录；仅签名区间 `317..381` 可变 |
| Signer Agent | 首类镜像与入口身份、独立策略、可注入 Provider |
| Core Gate | 原始 receipt 无权释放 Event；验证提交仅可消费一次 |
| 原生设备 | ATA LBA48、512 字节扇区、有界轮询、`FLUSH CACHE EXT` |
| 原生映射 | 每槽 128 个扇区；一个对齐的 256 扇区保留区间 |

```text
ATA IDENTIFY ──> 双槽扫描 ──> 链路 + 签名验证
                                │
                 ┌──────────────┴──────────────┐
                 ▼                             ▼
            GENESIS BOOT              RECOVERED(generation)
                 │                             │
                 └──────> 稳定 Resource <──── 一次性 Core proof
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

| 原生 Signer 边界 | 契约 |
| :--- | :--- |
| Core 身份 | `AgentImageKind::StateSigner` + `AgentEntryKind::StateSigner` |
| 镜像信任 | x86 kind `5`；独立 signer scope bit `4` |
| Provider ABI | 输入 285 字节 manifest，输出 64 字节签名，携带 policy generation |
| 算法策略 | 不可变选择 Ed25519 或 ECDSA P-256/SHA-256 |
| Package | 两个固定地址段、零重定位、输出权限 `0600` |
| 密钥归属 | Provider 保留持久状态密钥访问；Package 仅包含公开策略 |

```text
V18 HARDWARE SIGNER AGILITY
manifest          V1 legacy Ed25519 | V2 algorithm-bound
public key        Ed25519 / 32B | compressed SEC1 P-256 / 33B
signature         Ed25519 / 64B | IEEE P1363 low-S P-256 / 64B
failure policy    mismatch / malformed key / high-S -> fail closed
```

`ATA BACKEND` 完成 · `STATE SIGNER PACKAGE` 完成 · `SIGNER AGILITY` 完成

## `05 // AGENT CALL`

```text
┌─ REGISTER FRAME ────────────────────────────────────────────────┐
│ rax magic    rbx ABI       rcx operation / status              │
│ r8  Agent    rdi Task      rsi Image      r9 Nonce             │
│ r10..r15 + rbp             bounded payload                     │
└────────────────────────────────────────────────────────────────┘

解码 → 快照 → 鉴权 → 预检 → 修改 → 回复
```

| ID | 协议族 |
| ---: | :--- |
| `1..9` | 执行、Verification、Mailbox IPC |
| `10..20` | Resource、Capability、Task、Agent 生命周期 |
| `21..28` | Runtime Memory 与 Admission |
| `29..43` | 回收、压缩、Event 归档 |
| `44..52` | Namespace 绑定、解析、比较、修改、路径 |
| `53` | Agent Image Signer 策略轮换 |
| `54..55` | 持久归档 Prepare 与签名 Commit |

`TRANSPORT` 私有 call-data 页 · `POINTERS` 拒绝 · `REPLY` 规范寄存器

## `06 // 启动证据`

```text
PROFILE            V10 signed-v3
QEMU               debug + release
EVENTS             1..412 / 精确重放
AGENT CONTEXTS      11 个隔离上下文
DISPATCHES          35
FRAME OWNERSHIP     每 Agent 12..43
BOOT FRAME POOL     77 帧封存
```

| 证据面 | 信号 |
| :--- | :--- |
| 签名 Package | `AGENT_KERNEL_NATIVE_SIGNED_PACKAGE_OK` |
| 隔离 | `AGENT_KERNEL_MULTI_AGENT_ISOLATION_OK` |
| 上下文切换 | `AGENT_KERNEL_MULTI_AGENT_CONTEXT_SWITCH_OK` |
| Fault 恢复 | `AGENT_KERNEL_NATIVE_AGENT_FAULT_RESTART_OK` |
| Namespace 路径 | `AGENT_KERNEL_AGENT_CALL_NAMESPACE_MEMORY_PATH_OK` |
| 归档重放 | `AGENT_KERNEL_NATIVE_EVENT_ARCHIVE_REPLAY_OK` |
| Handoff | `SUPERVISOR_HANDOFF_READY` |

```text
V13 HOST RECEIPT
slot=A  generation=1  flush_epoch=3
archive=b72f0e90513d...e823449aff0d
```

```text
V14 ATA CONTRACT
commit path       390 次设备操作
cold scan         256 次扇区读取
fault boundaries  body write / footer flush / committed readback
```

```text
V15 DURABLE BOOT
request record    384 个 canonical 字节
recovery import   一次性 / 空白 Core / 溢出检查
boot profile      Disabled | ATA
bare target       x86_64-unknown-none
```

```text
V16 STATE SIGNER
call IDs          54 prepare / 55 commit
signature window  仅 bytes 317..381
session states    ready / prepared / faulted
closed loop       preflight / sign / ATA / release / cold recovery
```

```text
V17 FIRST-CLASS SIGNER
Core identity     StateSigner image + entry
trust scope       bit 4 / x86 image kind 5
native package    fixed address / 2 segments / 0 relocations
provider          external ABI / Package 不含持久状态密钥
```

```text
V18 SIGNER AGILITY
manifest          精确兼容 V1 / V2 显式算法
verification      Ed25519 verify_strict / P-256 SHA-256 low-S
policy            Provider + Package + Manifest 必须一致
closed loop       P-256 签名 / ATA 提交 / 断电 / 冷启动恢复
```

<details>
<summary><code>已验证镜像清单</code></summary>

| 原生镜像 | 格式 | Calls | 字节 | SHA-256 |
| :--- | :--- | ---: | ---: | :--- |
| Resource Manager | Signed Package v3 | 43 | 16,738 | `8fed932cf0a4...6699f9b3d` |
| Admission Supervisor | Capsule v1 | 44 | 4,115 | `5a657ca1ecde...9339078` |

</details>

## `07 // 构建启动`

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
    --image-key "$IMAGE_KEY" --provider-object "$PROVIDER_OBJECT" \
    --output "$STATE_SIGNER_PACKAGE" \
    --nonce 1 --archive-authority 2 --storage-authority 3 \
    --root 4 --storage 5 --through-sequence 64 \
    --call-data-generation 1 --policy-generation 1 \
    --state-signer-id "$STATE_SIGNER_ID"
```

```console
$ cargo check -p agent-kernel-x86_64 \
    --features bare-metal \
    --bin agent-kernel-x86_64 \
    --target x86_64-unknown-none
```

`TOOLCHAIN` Rust nightly · `EMULATOR` QEMU x86_64 · `TARGET` x86_64-unknown-none

## `08 // 源码树`

```text
crates/
├─ agent-kernel-core/    确定性内核模型
├─ agent-kernel/         no_std Facade
├─ agent-kernel-hal/     硬件请求协议
├─ agent-kernel-boot/    Bootstrap Profile
├─ agent-kernel-x86_64/  原生机器边界
├─ agent-kernel-image/   BIOS 镜像构建器
├─ agent-state-signer/   no_std 签名策略 Agent
└─ agent-supervisor/     宿主 Supervisor

docs/superpowers/{specs,plans}/
scripts/{run-qemu.sh,audit-agent-images.rb,build-state-signer-package.rb}
```

## `09 // 路线图`

```text
[done] Agent 原生权限 + 确定性 Event
[done] ring-3 隔离 + 每 Agent 独立地址空间
[done] 类型化 Namespace + 有界路径修改
[done] Package v3 + Ed25519 启动信任
[done] 运行时 signer 轮换 + Trust Policy Event
[done] 签名持久状态 + 双槽宿主恢复
[done] SMP + 同步 + TLB shootdown
[done] 原生 ATA PIO 适配器 + 签名冷启动恢复
[done] 验证持久启动 + Event 序列延续
[done] State Signer Agent + 原生归档 prepare/commit 调用
[done] 首类 Signer 身份 + 外部 Provider 原生 Package
[done] V1/V2 Signer 算法敏捷 + 低 S ECDSA P-256/SHA-256
[next] TPM/HSM 传输 + 密钥配置仪式
[next] QEMU 独立 ATA 镜像 + 模拟器断电验证
[next] Network + Graphics + USB + 形式化验证
```

| 轨道 | 记录 |
| :--- | :--- |
| 已验证基线 | [Signed Agent Package V10](docs/superpowers/specs/2026-07-21-signed-agent-package-v10-design.md) |
| Runtime 里程碑 | [SMP Runtime V12](docs/superpowers/specs/2026-07-23-smp-runtime-v12-design.md) |
| 持久协议 | [Signed Durable State V13](docs/superpowers/specs/2026-07-23-signed-durable-state-v13-design.md) |
| 原生存储 | [Native ATA Durable State V14](docs/superpowers/specs/2026-07-23-native-ata-durable-state-v14-design.md) |
| 当前里程碑 | [Hardware State Signer Agility V18](docs/superpowers/specs/2026-07-24-hardware-state-signer-agility-v18-design.md) |

## `10 // 项目`

| 字段 | 值 |
| :--- | :--- |
| 工程契约 | [`AGENTS.md`](AGENTS.md) |
| 许可证 | [`MIT`](LICENSE) |
| 状态 | 持续开发 |

```text
AGENT KERNEL // CONTROL PLANE FOR AUTONOMOUS MACHINES // 2026
```
