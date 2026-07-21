<div align="center">

# `AGENT KERNEL`

### `面向自主 Agent 的原生内核底座`

[English](README.md) / **简体中文**

<p>
  <img alt="Rust nightly" src="https://img.shields.io/badge/Rust-nightly-111111?logo=rust&amp;logoColor=white">
  <img alt="no_std" src="https://img.shields.io/badge/kernel-no__std-238636">
  <img alt="x86_64" src="https://img.shields.io/badge/arch-x86__64-0969da">
  <img alt="QEMU" src="https://img.shields.io/badge/proof-QEMU-f97316">
  <img alt="MIT" src="https://img.shields.io/badge/license-MIT-d0d7de">
</p>

<pre>
agent-kernel@ring0:~$ boot --profile package-v2
[ OK ] capability authority ........ online
[ OK ] isolated Agent address spaces online
[ OK ] segmented RX / R+NX packages  online
[ OK ] bounded ABS64 relocation ..... online
[ OK ] deterministic Event archive .. online
kernel://supervisor/handoff-ready
</pre>

</div>

```text
┌─ AGENT KERNEL // V9 ──────────────────────────────────────────┐
│ CORE      no_std / 无堆        TARGET    x86_64 裸机          │
│ MODE      ring 0 + ring 3      FORMAT    Package v2           │
│ AUTH      Capability           EVIDENCE  Tests / QEMU / ELF   │
│ STATUS    持续研发             LICENSE   MIT                  │
└───────────────────────────────────────────────────────────────┘
```

`模型` · `机器` · `PACKAGE` · `ABI` · `证据` · `启动`

## `00 / 内核信号`

| 通道 | 定义 |
| :--- | :--- |
| `主体` | Agent 身份绑定 Task、Image 与执行 Nonce |
| `权限` | 显式、可派生、可撤销的 Capability |
| `工作` | Intent → Action → Observation → Verification |
| `状态` | Resource、Namespace、Checkpoint、Rollback |
| `证据` | 有序 Event、归档摘要、精确重放 |

```text
AGENT ──提交──> CAPABILITY ──控制──> RESOURCE
  │                                      │
  └────────────── 生成 EVENT <───────────┘
```

## `01 / 原生模型`

| 内核不变量 | 强制契约 |
| :--- | :--- |
| `IDENTITY` | 调用继承当前已调度 Agent 上下文 |
| `AUTHORITY` | 受保护转换要求匹配的 Capability |
| `MUTATION` | 成功状态修改追加确定性 Event |
| `ISOLATION` | 每个原生 Agent 独占 CR3 根与 ring-3 帧 |
| `RECOVERY` | 故障路由、修复、重启、Checkpoint、Rollback |
| `I/O` | 授权 HAL 请求进入原生 IRQ 与端口路径 |

```text
IDENTITY    Agent / Task / Image / ExecutionContext
AUTHORITY   Capability / Scope / Operation / Delegation
WORK        Intent / Action / Observation / Verification
RECOVERY    Checkpoint / Rollback / Fault / Restart
STRUCTURE   Workspace / Namespace / Entry / Revision
EVIDENCE    Event / ArchiveDigest / Replay
```

## `02 / 机器路径`

```text
RING 3   verified Package ──> Agent ──> int 0x90 / IRQ / Fault
                                         │
──────────────────── 特权边界 ───────────┼──────────────────────
                                         ▼
RING 0   x86_64 入口 ──> ABI 解码 ──> 鉴权 ──> Facade
                                         │
                                         ▼
CORE     确定性转换 ──> 固定 Store ──> Event
                                         │
                                         ▼
HAL      不可变请求 ──> Driver Binding ──> Hardware
```

| Crate | 边界 |
| :--- | :--- |
| `agent-kernel-core` | 领域记录、固定 Store、确定性状态转换 |
| `agent-kernel` | `no_std` syscall 风格 Facade |
| `agent-kernel-x86_64` | 启动、CPU 帧、隔离、IRQ、Fault |
| `agent-kernel-hal` | 不可变设备请求协议 |
| `agent-supervisor` | 宿主模拟与用户空间编排 |

## `03 / PACKAGE V2`

```text
AGNTIMG\0 // Package v2
┌──────────────┬───────────────────┬──────────────────┐
│ header / 48B │ 2 segment records │ ABS64 records    │
├──────────────┴───────────────────┴──────────────────┤
│ code / 1..64 KiB / R+X                             │
├─────────────────────────────────────────────────────┤
│ rodata / 1..64 KiB / R+NX                          │
└─────────────────────────────────────────────────────┘
```

```text
SHA-256 校验
      ↓
分配精确帧 → 复制分段 → 通过私有代码别名写入重定位
      ↓
映射 code RX → 映射 rodata R+NX → 进入 ring 3
```

| 契约 | V9 边界 |
| :--- | :--- |
| `SEGMENTS` | 固定两段：code、rodata |
| `RELOCATIONS` | `0..64`，有序、无重叠、目标不跨页 |
| `SYMBOL` | `rodata_base + 非负 addend` |
| `PLACEHOLDER` | 不可变 Package 中的八个零字节 |
| `DIGEST` | 绑定重定位前的完整 Package |
| `LEGACY` | 迁移期间继续接收 Capsule v1 |

## `04 / 用户地址空间`

```text
0x4000_0000_0000..ffff  code / 16 页        RX
0x4000_0001_0000..ffff  rodata / 16 页      R + NX
0x4000_0002_0000        signal page         R + NX
0x4000_0002_1000        guard page          未映射
0x4000_0002_2000..5fff  stack / 4 页        RW + NX
0x4000_0002_6000        lazy page           按需映射
0x4000_0002_7000        runtime page        Capability 治理
0x4000_0002_8000..ffff  runtime / 8 页      Capability 治理
0x4000_0003_0000        call-data page      类型化固定记录
```

```text
FRAME IDENTITY
页表          4     code      1..16     rodata    0..16
signal        1     stack         4     lazy          1
call data     1     总持有    12..43     启动池        77
```

## `05 / AGENT CALL ABI`

```text
┌─ REGISTER FRAME ────────────────────────────────────────────────┐
│ rax magic    rbx ABI       rcx operation / status              │
│ r8  Agent    rdi Task      rsi Image      r9 Nonce             │
│ r10..r15 + rbp             bounded payload                     │
└────────────────────────────────────────────────────────────────┘
```

| Call ID | 协议族 |
| ---: | :--- |
| `1-9` | 执行、Verification、Mailbox IPC |
| `10-20` | Resource、Capability、Task、Agent 生命周期 |
| `21-28` | Runtime Memory 与 Admission |
| `29-43` | 回收、压缩、Event 归档 |
| `44-52` | Namespace 绑定、解析、比较、修改、路径 |

```text
TRANSPORT  私有 call-data 页 + 类型化记录
POINTERS   拒绝任意用户空间指针
IDENTITY   从已调度 CPU 上下文派生
REPLY      规范寄存器帧
ORDER      解码 → 鉴权 → 预检 → 修改
```

<details>
<summary><code>NAMESPACE // CALLS 44..52</code></summary>

```text
Workspace 1 --Cap A--> Mount(3) --Cap B--> Mount(8)
Workspace 8 --Cap C--> Mount(9) --Cap D--> Resource(3)

快照 → 解码 → 逐跳鉴权 → 比较 → rebind
```

| 路径 | 契约 |
| :--- | :--- |
| 绑定 / 解析 | 稳定 Entry ID 与有序解析 Event |
| 比较 / 修改 | 预期 revision 守卫与原子状态转换 |
| 有界路径 | 一至四段路径，每跳独立鉴权 |
| 内存传输 | 内核先快照，再解码与验证 |

</details>

## `06 / 原生能力矩阵`

| 子系统 | 原生机制 | QEMU 信号 |
| :--- | :--- | :--- |
| Package | v2 解析器、完整摘要、ABS64 重定位 | `NATIVE_SEGMENTED_PACKAGE_OK` |
| 隔离 | CR3、GDT/TSS/IDT、特权级切换 | `MULTI_AGENT_ISOLATION_OK` |
| 调度 | FIFO、PIT 抢占、CPU 帧恢复 | `MULTI_AGENT_CONTEXT_SWITCH_OK` |
| 故障 | `#UD`、`#GP`、`#PF`、修复、重启 | `NATIVE_AGENT_FAULT_RESTART_OK` |
| IPC | 阻塞 Mailbox、唤醒、确认、回收 | `NATIVE_MAILBOX_IPC_OK` |
| 内存 | 页/区域分配、First-Fit 复用、清零 | `NATIVE_MEMORY_CONCURRENCY_OK` |
| 管理器 | Resource、Task、Agent、Memory、Namespace | `NATIVE_RESOURCE_MANAGER_AGENT_OK` |
| Admission | 常驻 Supervisor、Permit、批量释放 | `NATIVE_RUNTIME_ADMISSION_COMMIT_OK` |
| Driver | UART IRQ、HAL 请求、原生 Invocation | `DRIVER_INVOCATION_FLOW_OK` |
| 审计 | SHA-256 归档链与精确重放 | `NATIVE_EVENT_ARCHIVE_REPLAY_OK` |

## `07 / 验证档案`

```text
QEMU TRANSCRIPT   Events 1..409      DISPATCH          35
AGENT CONTEXTS    11 个隔离上下文    NEXT SEQUENCE     410
CODE WINDOW       16 页 / 64 KiB     RODATA WINDOW     16 页 / 64 KiB
FRAMES PER AGENT  12..43             BOOT FRAME POOL   77 帧封存
EVENT STORE       峰值 375 / 最终 345 / 已归档 64
```

| 原生镜像 | 格式 | Calls | 镜像字节 | SHA-256 |
| :--- | :--- | ---: | ---: | :--- |
| Resource Manager | Package v2 | 43 | 16,634 | `14f09265ccbb...db7b646` |
| Admission Supervisor | Capsule v1 | 44 | 4,115 | `5a657ca1ecde...9339078` |

<details>
<summary><code>打开原始启动证据</code></summary>

```console
$ scripts/run-qemu.sh --release

[boot]       AGENT_KERNEL_QEMU_BOOT_OK
[package]    AGENT_KERNEL_NATIVE_SEGMENTED_PACKAGE_OK
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

## `08 / 构建 + 启动`

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

## `09 / 源码地图`

```text
crates/
├─ agent-kernel-core/    确定性模型 + 固定 Store
├─ agent-kernel/         no_std 内核 Facade
├─ agent-kernel-hal/     设备请求协议
├─ agent-kernel-boot/    Bootstrap + 容量配置
├─ agent-kernel-x86_64/  机器边界 + 原生执行
├─ agent-kernel-image/   BIOS 镜像构建器
└─ agent-supervisor/     宿主 Supervisor + 虚拟设备

docs/superpowers/
├─ specs/                架构记录
└─ plans/                里程碑计划
```

## `10 / 路线图`

```text
[x] Agent 原生权限模型
[x] ring-3 Agent + 每 Agent 独立地址空间
[x] 确定性 Event + 归档重放
[x] 类型化 Namespace + 有界路径修改
[x] 64 KiB code 窗口 + 精确帧所有权
[x] Package v2 + RX/R+NX 分段 + ABS64 重定位
[>] Package 签名 + 签名者信任策略
[ ] SMP + 同步 + TLB shootdown
[ ] Storage / Network / Graphics / USB
[ ] 签名持久状态 + 形式化验证
```

`当前 SPEC` · [`Segmented Agent Package V9`](docs/superpowers/specs/2026-07-21-segmented-agent-package-v9-design.md)

## `11 / 工程门禁`

| 门禁 | 要求 |
| :--- | :--- |
| `CONTRACT` | 遵循 [`AGENTS.md`](AGENTS.md) |
| `RED` | 运行时行为从失败契约测试开始 |
| `MODEL` | 保持显式权限与确定性 Event |
| `PROOF` | 通过聚焦测试、QEMU 转录与 ELF 审计 |

```text
AGENT KERNEL // MIT // COPYRIGHT 2026 RAN
```
