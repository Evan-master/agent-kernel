<div align="center">

# `AGENT KERNEL`

### `面向自主 Agent 的原生操作系统底座`

[English](README.md) / **简体中文**

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
│ MODEL     Agent 原生       CORE      no_std / 无堆             │
│ TARGET    x86_64 裸机      ABI       0.1 / 尚未稳定            │
│ MACHINE   BIOS / ring 0+3  EVIDENCE  tests / QEMU / ELF        │
│ STATUS    持续研发         LICENSE   MIT                       │
└────────────────────────────────────────────────────────────────┘
```

`模型` · `机器` · `CAPSULE` · `ABI` · `证据` · `启动`

## `00 / 内核信号`

Agent Kernel 是围绕 Agent、Capability、Resource、可验证工作、恢复机制与
确定性证据构建的原生操作系统内核。

```text
AGENT ──提交──> CAPABILITY ──控制──> RESOURCE
  │                                      │
  └────────────── 生成 EVENT <───────────┘
```

| 契约 | 内核不变量 |
| :--- | :--- |
| `IDENTITY` | 每次调用绑定 Agent、Task、Image 与 Nonce |
| `AUTHORITY` | 每项受保护操作均要求 Capability |
| `MUTATION` | 每次成功状态转换均生成有序 Event |
| `ISOLATION` | 每个原生 Agent 独占 CR3 根与 ring-3 上下文 |
| `RECOVERY` | Checkpoint、Rollback、故障路由、修复、重启 |
| `I/O` | 授权后的 HAL 请求进入原生 IRQ 与端口路径 |

## `01 / 原生模型`

```text
IDENTITY    Agent / Task / Image / ExecutionContext
AUTHORITY   Capability / Scope / Operation / Delegation
WORK        Intent / Action / Observation / Verification
RECOVERY    Checkpoint / Rollback / Fault / Restart
STRUCTURE   Workspace / Namespace / Entry / Revision
EVIDENCE    Event / ArchiveDigest / Replay
```

| 原语 | 内核职责 |
| :--- | :--- |
| `Agent` | 经过认证且可调度的权限主体 |
| `Capability` | 面向单个 Resource 的可派生、可撤销访问权 |
| `Intent` | 对目标工作的类型化声明 |
| `Task` | 绑定委托权限的可调度工作 |
| `Verification` | 执行后的独立可信状态转换 |
| `Checkpoint` | 由 Rollback 权限治理的恢复点 |
| `Event` | 成功状态修改生成的确定性证据 |
| `Namespace` | revision 绑定、Workspace Mount、有界路径 |

## `02 / 机器路径`

```text
RING 3   verified Capsule ──> Agent ──> int 0x90 / IRQ / Fault
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

| 层级 | 职责 |
| :--- | :--- |
| `agent-kernel-core` | 领域记录、固定 Store、确定性状态转换 |
| `agent-kernel` | `no_std` syscall 风格 Facade |
| `agent-kernel-x86_64` | 启动、CPU 帧、隔离、IRQ、Fault |
| `agent-kernel-hal` | 不可变设备请求协议 |
| `agent-supervisor` | 宿主模拟与用户空间编排 |

## `03 / AGENT CAPSULE`

```text
Capsule v1
┌──────────────┬──────────────┬──────────────────────────────────┐
│ magic / ABI  │ length / SHA │ fixed-layout x86_64 code         │
└──────────────┴──────────────┴──────────────────────────────────┘
        verify ──> allocate ──> map RX ──> enter ring 3
```

```text
用户地址空间
0x4000_0000_0000..ffff  code / 16 页        RX
0x4000_0001_0000        signal page         R + NX
0x4000_0001_1000        guard page          未映射
0x4000_0001_2000..5fff  stack / 4 页        RW + NX
0x4000_0001_6000        lazy page           按需映射
0x4000_0001_7000        runtime page        Capability 治理
0x4000_0001_8000..ffff  runtime / 8 页      Capability 治理
0x4000_0002_0000        call-data page      类型化固定记录
```

```text
V8 PROFILE
CODE WINDOW       64 KiB / 16 页有界 RX 窗口
PHYSICAL IDENTITY 12..27 帧 / 精确匹配 Capsule
CROSS-PAGE PROOF  Resource Manager 从第 5 页完成执行
POOL INVENTORY    76 帧封存库存 / 原子重组与复用
```

## `04 / AGENT CALL ABI`

```text
┌─ REGISTER FRAME ────────────────────────────────────────────────┐
│ rax magic    rbx ABI       rcx operation / status              │
│ r8  Agent    rdi Task      rsi Image      r9 Nonce             │
│ r10..r15 + rbp             bounded payload                     │
└────────────────────────────────────────────────────────────────┘
```

| Call ID | 协议族 |
| ---: | :--- |
| `1-9` | 执行、验证、Mailbox IPC |
| `10-20` | Resource、Capability、Task、Agent 生命周期 |
| `21-28` | Runtime Memory 与 Admission |
| `29-43` | 回收、压缩、Event 归档 |
| `44-52` | Namespace 绑定、解析、比较、修改、路径 |

```text
TRANSPORT  私有 call-data 页 + 类型化记录
POINTERS   拒绝任意用户空间指针
IDENTITY   从已调度 CPU 上下文派生
REPLY      规范寄存器帧
ORDER      解码 -> 鉴权 -> 预检 -> 修改
```

<details>
<summary><code>NAMESPACE // CALLS 44..52</code></summary>

| 路径 | 契约 |
| :--- | :--- |
| 绑定 / 解析 | 稳定 Entry ID 与有序解析 Event |
| 比较 / 修改 | 预期 revision 守卫与原子状态转换 |
| 有界路径 | 一至四段路径，每跳独立鉴权 |
| 内存传输 | 内核先快照，再解码与验证 |

```text
Workspace 1 --Cap A--> Mount(3) --Cap B--> Mount(8)
Workspace 8 --Cap C--> Mount(9) --Cap D--> Resource(3)

快照 -> 解码 -> 逐跳鉴权 -> 比较 -> rebind
```

</details>

## `05 / 原生能力矩阵`

| 子系统 | 原生机制 | QEMU 信号 |
| :--- | :--- | :--- |
| 隔离 | CR3、GDT/TSS/IDT、特权级切换 | `MULTI_AGENT_ISOLATION_OK` |
| 调度 | FIFO、PIT 抢占、CPU 帧恢复 | `MULTI_AGENT_CONTEXT_SWITCH_OK` |
| 故障 | `#UD`、`#GP`、`#PF`、修复、重启 | `NATIVE_AGENT_FAULT_RESTART_OK` |
| IPC | 阻塞 Mailbox、唤醒、确认、回收 | `NATIVE_MAILBOX_IPC_OK` |
| 内存 | 页/区域分配、First-Fit 复用、清零 | `NATIVE_MEMORY_CONCURRENCY_OK` |
| 管理器 | Resource、Task、Agent、Memory、Namespace | `NATIVE_RESOURCE_MANAGER_AGENT_OK` |
| Admission | 常驻 Supervisor、Permit、批量释放 | `NATIVE_RUNTIME_ADMISSION_COMMIT_OK` |
| Driver | UART IRQ、HAL 请求、原生 Invocation | `DRIVER_INVOCATION_FLOW_OK` |
| 审计 | SHA-256 归档链与精确重放 | `NATIVE_EVENT_ARCHIVE_REPLAY_OK` |

## `06 / 验证档案`

```text
QEMU TRANSCRIPT   Events 1..409
WORKSPACE TESTS   216 组 / 745 个通过
DISPATCH          35 次内核选择
AGENT CONTEXTS    11 个隔离上下文
CAPSULE WINDOW    16 页 / 64 KiB
FRAMES PER AGENT  12..27 / 活动代码页 1..16
BOOT FRAME POOL   76 帧封存 / 全量归还后清零
EVENT STORE       峰值 375 / 最终 345 / 已归档 64
NEXT SEQUENCE     410
```

| Native Capsule | Calls | 字节 | SHA-256 |
| :--- | ---: | ---: | :--- |
| Resource Manager | 43 | 16,480 | `3a8764b8c986...bdca8dc6e` |
| Admission Supervisor | 44 | 4,115 | `e09598b938db...c3bc04b01` |

<details>
<summary><code>打开原始启动证据</code></summary>

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

## `07 / 构建 + 启动`

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

## `08 / 源码地图`

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

## `09 / 路线图`

```text
[x] Agent 原生权限模型
[x] ring-3 Capsule + 每 Agent 独立地址空间
[x] 确定性 Event + 归档重放
[x] 类型化 Namespace + 有界路径修改
[x] 十六页 Agent Capsule + 第五页执行
[x] 按镜像尺寸分配可执行帧
[>] 分段软件包 + 重定位 + 签名
[ ] SMP + 同步 + TLB shootdown
[ ] Storage / Network / Graphics / USB
[ ] 签名持久状态 + 形式化验证
```

`当前 SPEC` · [`Expanded Agent Capsule V8`](docs/superpowers/specs/2026-07-21-expanded-agent-capsule-v8-design.md)

## `10 / 工程门禁`

| 门禁 | 要求 |
| :--- | :--- |
| `CONTRACT` | 遵循 [`AGENTS.md`](AGENTS.md) |
| `RED` | 运行时行为从失败契约测试开始 |
| `MODEL` | 保持显式权限与确定性 Event |
| `PROOF` | 通过聚焦测试、QEMU 转录与 ELF 审计 |

```text
AGENT KERNEL // MIT // COPYRIGHT 2026 RAN
```
