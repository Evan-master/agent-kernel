<h1 align="center"><code>AGENT KERNEL</code></h1>

<p align="center">
  Agent 原生权限模型 / 隔离执行 / 确定性证据
</p>

<p align="center">
  <a href="README.md">English</a> / <strong>简体中文</strong>
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
project  : 内核持续开发
abi      : 0.1 / 尚未稳定
core     : no_std / 固定容量 / 无堆
machine  : x86_64 / BIOS / ring 0 + ring 3
proof    : 测试 + QEMU 转录 + ELF 审计
```

[`模型`](#01--原生模型) / [`机器路径`](#02--机器路径) /
[`ABI`](#03--agent-call-abi) / [`验证`](#06--验证档案) /
[`启动`](#07--构建--启动) / [`路线图`](#09--路线图)

## 00 / 系统信号

| 通道 | 内核契约 |
| --- | --- |
| 身份 | 每次调用绑定已调度的 Agent、Task、Image 与 Nonce |
| 权限 | 每次 Resource 操作提交显式 Capability |
| 修改 | 每次成功状态转换生成有序 Event |
| 恢复 | Checkpoint、Rollback、故障路由、修复、重启 |
| 隔离 | 每个 Agent 拥有独立 CR3 根与 ring-3 执行上下文 |
| I/O | 授权后的 HAL 请求进入原生中断与端口路径 |

## 01 / 原生模型

```text
AGENT --提交--> CAPABILITY --控制--> RESOURCE
  |                                  |
  +------------- 生成 EVENT <--------+
```

```text
identity     Agent / Task / Image / Execution Context
authority    Capability / Scope / Operations / Delegation
work         Intent / Action / Observation / Verification
recovery     Checkpoint / Rollback / Fault / Restart
structure    Workspace / Namespace / Entry / Revision
evidence     Event / Archive Digest / Replay
```

| 原语 | 职责 |
| --- | --- |
| `Agent` | 经过认证的权限主体，保存可调度状态 |
| `Capability` | 面向单个 Resource 的作用域与操作集合，可派生、可撤销 |
| `Intent` | 对目标工作的类型化声明 |
| `Task` | 绑定委托权限的可调度工作 |
| `Verification` | 执行完成后的独立可信状态转换 |
| `Checkpoint` | 由 Rollback 权限管理的恢复点 |
| `Event` | 成功状态修改产生的确定性证据 |
| `Namespace` | revision 绑定、显式 Workspace Mount、有界路径 |

```text
模型运行时 / Prompt / 规划 / 外部适配器  -> 用户空间
身份 / 权限 / 调度 / 隔离 / 审计         -> 内核空间
```

## 02 / 机器路径

```text
RING 3   Verified Capsule -> Agent -> int 0x90 / IRQ / Fault
                                  |
------------------ 特权边界 -------------------------------
                                  v
RING 0   x86_64 入口 -> ABI 解码 -> 鉴权 -> no_std Facade
                                  |
                                  v
CORE     确定性转换 -> 固定 Store -> Event
                                  |
                                  v
HAL      不可变请求 -> Driver Binding -> Hardware
```

| 层 | 职责 |
| --- | --- |
| `agent-kernel-core` | 领域记录、固定 Store、确定性状态转换 |
| `agent-kernel` | `no_std` syscall 风格 Facade |
| `agent-kernel-x86_64` | 启动、特权边界、CPU 帧、IRQ、Fault |
| `agent-kernel-hal` | 不可变设备请求协议 |
| `agent-supervisor` | 宿主模拟与用户空间编排 |

## 03 / Agent Call ABI

```text
rax = magic       rbx = ABI version      rcx = operation / status
r8  = Agent       rdi = Task             rsi = Image
r9  = Nonce       r10..r15, rbp = bounded operation payload
```

| ID | 协议族 |
| ---: | --- |
| `1-9` | 执行、验证、Mailbox IPC |
| `10-20` | Resource、Capability、Task、Agent 生命周期 |
| `21-28` | Runtime Memory 与 Admission |
| `29-43` | 回收、压缩、Event 归档 |
| `44-50` | Namespace 绑定、解析、修改、比较、有界路径 |

```text
userspace pointers : 0
identity source    : 已调度 CPU 上下文
reply shape        : 规范寄存器帧
failure rule       : 解码 / 鉴权 / 预检先于状态修改
```

<details>
<summary><strong>ABI 不变量</strong></summary>

- Core 再次检查 Capability 作用域与操作位。
- 事务预检容量、活引用与 Event 槽位。
- 规范回复清理无关寄存器。
- Capsule、CPU 帧与转录检查覆盖原生执行。

</details>

## 04 / Namespace 协议

| Call | ID | 权限 | 状态转换 |
| --- | ---: | --- | --- |
| `BindNamespaceEntry` | 44 | `Act` | 分配单调递增 Entry ID |
| `ResolveNamespaceEntry` | 45 | `Observe` | 返回记录并生成解析证据 |
| `RebindNamespaceEntry` | 46 | `Act` | 替换对象并推进 revision |
| `RetireNamespaceEntry` | 47 | `Rollback` | 删除稳定 Entry 并归还槽位 |
| `CompareAndRebindNamespaceEntry` | 48 | `Act` | 在预期 revision 上替换 |
| `CompareAndRetireNamespaceEntry` | 49 | `Rollback` | 在预期 revision 上回收 |
| `ResolveNamespacePath` | 50 | 每跳 `Observe` | 解析一段或两段原生路径 |

```text
Workspace 1 -- Key 1 / Capability A --> Mount(Workspace 3)
Workspace 3 -- Key 2 / Capability B --> terminal Entry
```

## 05 / 运行矩阵

| 子系统 | 原生路径 | QEMU 证据 |
| --- | --- | --- |
| 隔离 | CR3 根、GDT/TSS/IDT、特权级切换 | `MULTI_AGENT_ISOLATION_OK` |
| 调度 | FIFO、PIT 抢占、CPU 帧恢复 | `MULTI_AGENT_CONTEXT_SWITCH_OK` |
| 故障 | `#UD`、`#GP`、`#PF`、路由、修复、重启 | `NATIVE_AGENT_FAULT_RESTART_OK` |
| IPC | 阻塞 Mailbox、唤醒、确认、回收 | `NATIVE_MAILBOX_IPC_OK` |
| 内存 | 页/区域分配、First-Fit 复用、清零 | `NATIVE_MEMORY_CONCURRENCY_OK` |
| 管理器 | Resource、Capability、Task、Agent、Memory、Namespace | `NATIVE_RESOURCE_MANAGER_AGENT_OK` |
| Admission | 常驻 Supervisor、Permit、批量释放 | `NATIVE_RUNTIME_ADMISSION_COMMIT_OK` |
| Driver | UART IRQ、HAL 请求、原生 Invocation | `DRIVER_INVOCATION_FLOW_OK` |
| 审计 | Event Log、SHA-256 归档、精确重放 | `NATIVE_EVENT_ARCHIVE_REPLAY_OK` |

## 06 / 验证档案

| 指标 | 值 |
| --- | ---: |
| 目标 | `x86_64-unknown-none` |
| 隔离 Agent 上下文 | 11 |
| 内核选择 Dispatch | 35 |
| Resource Manager Calls / CR3 switches | `38 / 76` |
| Admission Supervisor Calls / CR3 switches | `44 / 88` |
| Namespace 容量 / 最终占用 | `2 / 1` |
| 实时 Event 容量 / 峰值 | `362 / 362` |
| 已归档 Event | 64 |
| 最终实时 Event / 下一序列 | `332 / 397` |
| 完整转录 | Events `1..396` |

| Native Capsule | Calls | 字节 | SHA-256 |
| --- | ---: | ---: | --- |
| Resource Manager | 38 | 3,789 | `24d6a22464c9...08bdcc1a` |
| Admission Supervisor | 44 | 4,115 | `f6c4efffe3c5...6f72f3f2` |

<details>
<summary><strong>摘要 / 终端 Event 窗口</strong></summary>

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

## 07 / 构建 + 启动

```bash
git clone https://github.com/Evan-master/agent-kernel.git
cd agent-kernel

cargo test --workspace
cargo run -p agent-supervisor
```

```bash
# 裸机转录门槛
scripts/run-qemu.sh
scripts/run-qemu.sh --release
```

```bash
# 裸机编译门槛
cargo check \
  -p agent-kernel-x86_64 \
  --features bare-metal \
  --bin agent-kernel-x86_64 \
  --target x86_64-unknown-none
```

工具链：`rustup`、仓库固定 nightly、LLVM tools、`x86_64-unknown-none`、QEMU。

## 08 / 仓库地图

```text
crates/
|- agent-kernel-core/    确定性 no_std 模型 + Store
|- agent-kernel/         no_std syscall 风格 Facade
|- agent-kernel-hal/     不可变设备请求协议
|- agent-kernel-boot/    Bootstrap + 容量配置
|- agent-kernel-x86_64/  启动 + 隔离 + IRQ + Fault + Agent Call
|- agent-kernel-image/   BIOS 镜像构建器
`- agent-supervisor/     宿主 Supervisor + 虚拟设备后端

docs/superpowers/
|- specs/                已确认架构记录
`- plans/                里程碑实现计划
```

## 09 / 路线图

| 方向 | 当前 | 下一阶段 |
| --- | --- | --- |
| Namespace | Mount 与有界遍历 | 三跳/四跳用户内存传输 |
| Memory | 私有页表、页/区域复用 | 动态页表增长 |
| Scheduling | 单核 FIFO 与 PIT | SMP、同步、TLB shootdown |
| Durability | 有界 SHA-256 归档链 | 崩溃一致的签名存储 |
| Devices | UART 与 Port I/O | Storage、Network、Graphics、USB |
| Agent 软件 | 定宽 Capsule | Package 格式与生产加载器 |
| Assurance | 测试、QEMU 转录、ELF 审计 | 加固与形式化验证 |

最新设计记录：
[`Native Namespace Hierarchy V3`](docs/superpowers/specs/2026-07-20-native-namespace-hierarchy-v3-design.md)

## 参与贡献

- 阅读 [`AGENTS.md`](AGENTS.md)。
- 运行时行为从失败测试开始。
- 保持显式权限与确定性 Event。
- 提交聚焦测试与对应 QEMU 证据。

## 许可证

[`MIT`](LICENSE) / Copyright 2026 Ran
