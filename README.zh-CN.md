# Agent Kernel

[English](README.md) | **简体中文**

Agent Kernel 是一个用 Rust 编写的 Agent 原生操作系统内核。它以 Agent、资源、
能力、意图、任务、事件、验证和回滚作为主要内核对象，其架构不依赖 Linux、
命令行自动化或 POSIX 兼容层。

> **开发状态：** 项目处于持续内核开发阶段。独立的 x86_64 目标已经能够在
> QEMU 中启动并运行相互隔离的 ring-3 Agent Capsule；ABI 与架构仍在演进，
> 生产级稳定性将在后续阶段建立。

## 系统模型

传统操作系统主要向程序提供进程、文件、套接字和用户等抽象。一个真正以
Agent 为中心的系统需要不同的控制面：

- **Agent** 是内核可见的执行主体和权限主体。
- **资源（Resource）** 是 Agent 可以控制的任何内核对象。
- **能力（Capability）** 明确规定哪个 Agent 能对哪个资源执行哪些操作；
  权限可以收窄、派生和撤销。
- **意图（Intent）** 描述想完成的工作，**任务（Task）** 是可调度执行单元。
- **验证（Verification）** 与“执行成功”分离，成功不自动等于结果可信。
- **检查点（Checkpoint）** 和 **回滚（Rollback）** 是一等生命周期操作。
- 每次成功修改都会生成有序 **事件（Event）**，用于审计与重放。

原生模型中不存在环境式的“默认超级用户”。Agent 可以拥有很高权限，但这些
权限必须由明确的 Capability 表达，并且始终能在事件日志中观察到。

## 当前实现

参考 BIOS/QEMU 配置不依赖 Linux 或其他宿主操作系统作为内核底座，当前包括：

- 永久 GDT、TSS、IDT、ring-0/ring-3 边界和每个 Agent 独立的 CR3 页表根；
- 六个隔离的原生 Agent 上下文：两个 Worker、一个 Verifier、一个 Fault
  Worker、一个 Fault Handler 和一个 Resource Manager；
- 由内核选择的 FIFO 调度、真实 PIT 定时器抢占，以及跨恢复过程完整持有 CPU 帧；
- 由 SHA-256 绑定的定长 Agent Image Capsule，以及 Worker、Verifier、
  FaultHandler、Supervisor 类型化入口；
- 仅使用寄存器、不接受用户态指针的版本化 Agent Call ABI；
- 阻塞式邮箱发送、接收、确认与唤醒，主动 Yield，任务结果、目标限定验证和完成；
- 对 ring-3 `#UD`、`#GP`、`#PF` 的故障隔离；来自内核态的异常仍然直接失败；
- 将缺页故障按策略路由给真实 ring-3 Fault Handler，再通过 Capability
  限定的方式修复保留页，并从同一故障帧继续执行；
- 真实 ring-3 Resource Manager：使用派生的 `Act` 权限创建子 Service，
  将收窄后的 `Observe` 权限派生给另一个 Agent，通过来源的 `Delegate`
  权限撤销这个直接子 Capability，再以 `Rollback` 权限退休该 Service；
- 从 UART 中断、端点解析、不可变 HAL 请求、Port I/O、结果记录到 Driver
  Invocation 完成的内核授权驱动链路。

参考验证配置强制满足以下确定性不变量：

| 证据 | 数量 |
| --- | ---: |
| 注册 Agent | 8 |
| 原生 ring-3 完成上下文 | 6 |
| 内核选择的 Dispatch | 23 |
| 真实物理时间片到期 | 10 |
| 被隔离的 Agent 故障 | 4 |
| Resource Manager 执行后的资源 | 2 |
| Resource Manager 执行后的 Capability | 12 |
| Driver 完成后的有序内核事件 | 171 |

`scripts/run-qemu.sh` 会逐条校验事件顺序，同时拒绝缺失标记、多余事件、异常的
QEMU 退出状态以及任何 fail-closed 启动路径。

## 架构

```mermaid
flowchart TB
    Capsule["已验证的 Agent Capsule"] --> Ring3["隔离的 ring-3 Agent"]
    Ring3 -->|"Agent Call / 中断 / 故障"| X86["x86_64 边界适配层"]
    X86 --> Facade["agent-kernel 系统调用门面"]
    Facade --> Core["agent-kernel-core 确定性模型"]
    Core --> Stores["固定容量对象存储"]
    Core --> Events["有序事件日志"]
    Core --> Scheduler["Task 与 Driver 调度器"]
    Scheduler --> X86
    Core --> HAL["不可变 HAL 请求"]
    HAL --> Device["架构或宿主设备后端"]
    Supervisor["用户态 Supervisor / 未来模型运行时"] --> Facade
```

内核保持小型、确定性和可检查。用户态 Supervisor 负责 LLM 推理、Prompt、
远程模型调用和高级规划；内核空间负责确定性执行与权限原语。

## 工作区结构

| Crate | 职责 |
| --- | --- |
| `agent-kernel-core` | `no_std` AgentOS 对象模型、授权、生命周期、调度与事件 |
| `agent-kernel` | 核心模型之上的 `no_std` 系统调用式门面 |
| `agent-kernel-hal` | 由内核授权的不可变设备请求协议 |
| `agent-kernel-boot` | 确定性 Bootstrap 交接与固定容量配置 |
| `agent-kernel-x86_64` | 裸机 x86_64 启动、隔离、中断、故障、Agent Call 与 QEMU 验证 |
| `agent-kernel-image` | 在宿主机生成 BIOS 磁盘镜像的工具 |
| `agent-supervisor` | 宿主侧用户态模拟器与虚拟设备后端 |

所有内核存储都使用固定容量。核心层和门面层不使用堆分配、宿主文件、Socket、
线程、随机数或隐藏的全局可变状态。

## Agent Call ABI

Agent Call 通过固定寄存器帧跨越 ring-3 边界。任何修改状态的请求在进入门面层
之前，都必须与调度器持有的 Agent、Task、Image 和 Nonce 身份完全匹配。

| 操作 | ID | 用途 |
| --- | ---: | --- |
| `DescribeContext` | 1 | 建立可信执行身份和 Nonce |
| `Yield` | 2 | 主动把正在运行的 Task 放回队列 |
| `CompleteTask` | 3 | 完成已认证的 Task |
| `SubmitTaskResult` | 4 | 保存定宽 Task 结果 |
| `InspectTaskResult` | 5 | 检查一个被授权目标的结果 |
| `VerifyTask` | 6 | 提交目标限定的验证结果 |
| `SendMessage` | 7 | 发送引用类型化内核对象的消息 |
| `ReceiveMessage` | 8 | 接收消息，或原子进入邮箱等待 |
| `AcknowledgeMessage` | 9 | 确认已接收消息 |
| `CreateResource` | 10 | 使用明确的父资源权限创建子资源 |
| `RetireResource` | 11 | 使用资源的 `Rollback` Capability 将其退休 |
| `DeriveCapability` | 12 | 为另一个已注册 Agent 派生收窄后的来源权限 |
| `RevokeDerivedCapability` | 13 | 通过带 `Delegate` 的来源撤销一个直接子 Capability |

原生资源 ABI 当前接受 Workspace、Memory、Service、Network 和 Device。
未知类型、未知操作位、零句柄、过期 Nonce、错误身份或非零保留寄存器都会直接拒绝。

## 快速开始

### 环境要求

- 通过 `rustup` 安装 Rust；
- 仓库指定的 nightly、`rust-src`、LLVM tools 和 `x86_64-unknown-none`
  target。使用 rustup 管理的 Cargo 时，会按 `rust-toolchain.toml` 自动安装；
- 运行独立 x86_64 验证目标所需的 `qemu-system-x86_64`。

macOS 可通过 Homebrew 安装 QEMU：

```bash
brew install qemu
```

### 构建与测试

```bash
git clone https://github.com/Evan-master/agent-kernel.git
cd agent-kernel

cargo fmt --check
cargo test --workspace
cargo run -p agent-supervisor
```

### 运行 x86_64 验证目标

```bash
scripts/run-qemu.sh
scripts/run-qemu.sh --release
```

脚本会构建裸机目标、生成 BIOS 镜像、启动 QEMU、检查完整串口记录、要求恰好
171 个事件，并把内核 debug-exit 状态也作为契约的一部分。成功运行的结尾是：

```text
AGENT_KERNEL_NATIVE_RESOURCE_MANAGER_AGENT_OK
AGENT_KERNEL_NATIVE_CAPABILITY_MANAGER_OK
AGENT_KERNEL_DRIVER_INVOCATION_FLOW_OK
event[171] driver_invocation_completed
SUPERVISOR_HANDOFF_READY
```

## 权限与失败模型

- 资源访问始终通过明确的 Capability。
- Task 范围的 Capability 不能被悄悄当作通用 Resource 权限。
- 派生权限不能超过来源权限；祖先 Capability 被撤销后，后代权限也失效。
- 架构适配层只能调用公开门面 API，不能直接修改核心存储。
- 多记录修改会先完成容量检查，因此失败保持原子性。
- 已退休对象仍可查询审计，但不能继续作为活跃对象使用。
- Capsule、Agent Call、CPU 帧、事件序列或物理所有权证据不匹配时，系统按
  fail-closed 策略终止验证。

高权限 Agent 的所有广泛权限都保持明确、可组合、可撤销、可审计。

## 已实现与待实现

### 已实现

- Agent、Resource、Capability、Intent、Task、Action、Observation、
  Verification、Checkpoint、Rollback、Message、Fault、Driver、Memory Cell、
  Namespace 和 Event 原语；
- Capability 授予与收窄、Task 委派、来源撤销传播、受认证的直接子权限撤销、
  资源所有权和资源退休；
- 固定容量调度、等待/唤醒、邮箱 IPC、故障策略、镜像验证、运行时准入和 Driver
  Invocation 生命周期；
- 裸机 x86_64 隔离、时钟抢占、故障隔离/恢复、原生资源生命周期 Agent Call
  和跨 Agent 权限控制。

### 后续规划

- 通用物理/虚拟内存管理器和动态用户态分配器；
- SMP 调度、多核同步和硬件 TLB Shootdown；
- 通用存储、网络、图形、USB 或真实硬件支持；
- 超出当前定长 Capsule 原型的 Agent 包和应用格式；
- 生产级用户态 Supervisor、模型运行时和策略规划器；
- POSIX、Linux 或 Windows 兼容层；
- 生产安全加固、形式化验证和稳定 ABI 承诺。

最新里程碑的完整契约见 [Capability Manager 设计](docs/superpowers/specs/2026-07-17-x86-native-capability-manager-v0-design.md)
和 [实现计划](docs/superpowers/plans/2026-07-17-x86-native-capability-manager-v0.md)。
历史设计记录保留在 `docs/superpowers/specs/`。

## 参与贡献

修改代码前请阅读 [AGENTS.md](AGENTS.md)。提交内容必须保持既有内核架构和验证
契约，尤其注意：

- 核心模型保持 Agent 原生，POSIX 支持限定在兼容层；
- 保持 `no_std`、确定性、固定容量存储和明确事件；
- 新运行时行为必须先写失败测试；
- 所有特权修改都经过明确的 Capability 检查；
- 发布前运行工作区测试、Supervisor 和相关 QEMU 验证。

## 许可证

[MIT](LICENSE)
