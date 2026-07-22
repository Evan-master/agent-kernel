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
$ ak boot --profile signed-v3
[00] identity .............. bound
[01] capability graph ...... online
[02] Ed25519 trust policy .. verified
[03] ring-3 agents ......... isolated
[04] event archive ......... sealed
kernel://supervisor/handoff-ready
</pre>

</div>

```text
┌─ SYSTEM STATUS ─────────────────────────────────────────────────┐
│ VERIFIED   V10 / QEMU debug + release   HEAD   V11 trust policy │
│ KERNEL     no_std / 无堆                 ISA    x86_64           │
│ MODE       ring 0 + ring 3              ABI    Agent Call       │
│ IMAGE      Signed Package v3            AUTH   Capability       │
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
| `agent-kernel-x86_64` | 启动、分页、特权切换、IRQ、原生执行 |
| `agent-kernel-hal` | 不可变设备请求协议 |
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
| 调度 | FIFO 派发、PIT 抢占、CPU Frame 恢复 |
| 隔离 | 每 Agent 页表、GDT/TSS/IDT、ring-3 入口 |
| 恢复 | `#UD`、`#GP`、`#PF`、修复、重启、回滚 |
| IPC | 阻塞 Mailbox、唤醒、确认、回收 |
| 内存 | 页/区域分配、First-Fit 复用、清零 |
| I/O | Capability 授权的 HAL 请求、IRQ、端口访问 |

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

## `04 // AGENT CALL`

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

`TRANSPORT` 私有 call-data 页 · `POINTERS` 拒绝 · `REPLY` 规范寄存器

## `05 // 启动证据`

```text
PROFILE            V10 signed-v3
QEMU               debug + release
EVENTS             1..409 / 精确重放
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

<details>
<summary><code>已验证镜像清单</code></summary>

| 原生镜像 | 格式 | Calls | 字节 | SHA-256 |
| :--- | :--- | ---: | ---: | :--- |
| Resource Manager | Signed Package v3 | 43 | 16,738 | `8fed932cf0a4...6699f9b3d` |
| Admission Supervisor | Capsule v1 | 44 | 4,115 | `5a657ca1ecde...9339078` |

</details>

## `06 // 构建启动`

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
```

```console
$ cargo check -p agent-kernel-x86_64 \
    --features bare-metal \
    --bin agent-kernel-x86_64 \
    --target x86_64-unknown-none
```

`TOOLCHAIN` Rust nightly · `EMULATOR` QEMU x86_64 · `TARGET` x86_64-unknown-none

## `07 // 源码树`

```text
crates/
├─ agent-kernel-core/    确定性内核模型
├─ agent-kernel/         no_std Facade
├─ agent-kernel-hal/     硬件请求协议
├─ agent-kernel-boot/    Bootstrap Profile
├─ agent-kernel-x86_64/  原生机器边界
├─ agent-kernel-image/   BIOS 镜像构建器
└─ agent-supervisor/     宿主 Supervisor

docs/superpowers/{specs,plans}/
scripts/{run-qemu.sh,audit-agent-images.rb}
```

## `08 // 路线图`

```text
[done] Agent 原生权限 + 确定性 Event
[done] ring-3 隔离 + 每 Agent 独立地址空间
[done] 类型化 Namespace + 有界路径修改
[done] Package v3 + Ed25519 启动信任
[work] 运行时 signer 轮换 + Trust Policy Event
[next] SMP + 同步 + TLB shootdown
[next] Storage + Network + Graphics + USB
[next] 签名持久状态 + 形式化验证
```

| 轨道 | 记录 |
| :--- | :--- |
| 已验证基线 | [Signed Agent Package V10](docs/superpowers/specs/2026-07-21-signed-agent-package-v10-design.md) |
| 当前里程碑 | [Runtime Trust Policy V11](docs/superpowers/specs/2026-07-22-runtime-trust-policy-v11-design.md) |

## `09 // 项目`

| 字段 | 值 |
| :--- | :--- |
| 工程契约 | [`AGENTS.md`](AGENTS.md) |
| 许可证 | [`MIT`](LICENSE) |
| 状态 | 持续开发 |

```text
AGENT KERNEL // CONTROL PLANE FOR AUTONOMOUS MACHINES // 2026
```
