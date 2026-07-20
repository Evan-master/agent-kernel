<div align="center">

# `AGENT KERNEL`

`权限 // 隔离 // 确定性证据`

[English](README.md) / **简体中文**

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
| STATUS    持续开发            | ABI       0.1 / 尚未稳定       |
| CORE      no_std / 无堆       | TARGET    x86_64 裸机          |
| MACHINE   BIOS / ring 0 + 3   | PROOF     test/QEMU/ELF        |
+----------------------------------------------------------------+
```

<p align="center">
  <a href="#01--原生模型"><code>模型</code></a> /
  <a href="#02--机器路径"><code>机器</code></a> /
  <a href="#03--agent-call-abi"><code>ABI</code></a> /
  <a href="#04--类型化-namespace"><code>NAMESPACE</code></a> /
  <a href="#06--验证档案"><code>验证</code></a> /
  <a href="#07--构建--启动"><code>启动</code></a> /
  <a href="#09--路线图"><code>路线图</code></a>
</p>

## 00 // 内核契约

| 信号 | 强制契约 |
| :--- | :--- |
| `IDENTITY` | 调用帧绑定已调度的 Agent、Task、Image 与 Nonce |
| `AUTHORITY` | Resource 操作提交显式 Capability |
| `MUTATION` | 成功状态转换生成有序 Event |
| `RECOVERY` | Checkpoint、Rollback、故障路由、修复、重启 |
| `ISOLATION` | 每个 Agent 拥有独立 CR3 根与 ring-3 执行上下文 |
| `I/O` | 授权后的 HAL 请求进入原生 IRQ 与端口路径 |

## 01 // 原生模型

```text
 AGENT ---- 提交 ----> CAPABILITY ---- 控制 ----> RESOURCE
   |                                                  |
   +---------------- 生成 EVENT <---------------------+
```

```text
IDENTITY   Agent / Task / Image / Execution Context
AUTHORITY  Capability / Scope / Operation / Delegation
WORK       Intent / Action / Observation / Verification
RECOVERY   Checkpoint / Rollback / Fault / Restart
STRUCTURE  Workspace / Namespace / Entry / Revision
EVIDENCE   Event / Archive Digest / Replay
```

| 原语 | 内核职责 |
| :--- | :--- |
| `Agent` | 经过认证的权限主体，保存可调度状态 |
| `Capability` | 面向单个 Resource 的操作集合，可派生、可撤销 |
| `Intent` | 对目标工作的类型化声明 |
| `Task` | 绑定委托权限的可调度工作 |
| `Verification` | 执行完成后的独立可信状态转换 |
| `Checkpoint` | 由 Rollback 权限管理的恢复点 |
| `Event` | 成功状态修改产生的确定性证据 |
| `Namespace` | revision 绑定、Workspace Mount、有界路径 |

```text
USER SPACE  模型运行时 | Prompt | 规划 | 外部适配器
----------- int 0x90 / IRQ / Fault --------------------
KERNEL      身份 | 权限 | 调度 | 隔离 | 审计
```

## 02 // 机器路径

```text
RING 3    verified Capsule -> Agent -> int 0x90 / IRQ / Fault
                                      |
------------------- 特权边界 ---------|------------------------
                                      v
RING 0    x86_64 入口 -> ABI 解码 -> 鉴权 -> Facade
                                      |
                                      v
CORE      确定性转换 -> 固定 Store -> Event
                                      |
                                      v
HAL       不可变请求 -> Driver Binding -> Hardware
```

| Crate | 职责 |
| :--- | :--- |
| `agent-kernel-core` | 领域记录、固定 Store、确定性状态转换 |
| `agent-kernel` | `no_std` syscall 风格 Facade |
| `agent-kernel-x86_64` | 启动、特权边界、CPU 帧、IRQ、Fault |
| `agent-kernel-hal` | 不可变设备请求协议 |
| `agent-supervisor` | 宿主模拟与用户空间编排 |

## 03 // Agent Call ABI

```text
+---------------------------- CALL FRAME -------------------------+
| rax  magic       | rbx  ABI version | rcx  operation / status  |
| r8   Agent       | rdi  Task        | rsi  Image               |
| r9   Nonce       | r10..r15, rbp    | bounded payload          |
+-----------------------------------------------------------------+
```

| ID | 协议族 |
| ---: | :--- |
| `1-9` | 执行、验证、Mailbox IPC |
| `10-20` | Resource、Capability、Task、Agent 生命周期 |
| `21-28` | Runtime Memory 与 Admission |
| `29-43` | 回收、压缩、Event 归档 |
| `44-52` | Namespace 绑定、解析、比较、修改、有界路径 |

```text
TRANSPORT  固定私有 call-data 页 + 类型化记录
POINTERS   拒绝任意用户空间指针
IDENTITY   来自已调度 CPU 上下文
REPLY      规范寄存器帧
ORDER      解码 -> 鉴权 -> 预检 -> 修改
```

<details>
<summary><code>ABI_INVARIANTS</code></summary>

| 门槛 | 检查项 |
| :--- | :--- |
| Core | Capability 作用域与操作位 |
| Transaction | 容量、活引用、Event 槽位 |
| Reply | 清理无关寄存器 |
| Native proof | Capsule、CPU 帧、转录 |

</details>

## 04 // 类型化 Namespace

| Call | ID | 权限 | 状态转换 |
| :--- | ---: | :--- | :--- |
| `BindNamespaceEntry` | 44 | `Act` | 分配单调递增 Entry ID |
| `ResolveNamespaceEntry` | 45 | `Observe` | 返回记录并生成解析 Event |
| `RebindNamespaceEntry` | 46 | `Act` | 替换对象并推进 revision |
| `RetireNamespaceEntry` | 47 | `Rollback` | 删除稳定 Entry 并归还槽位 |
| `CompareAndRebindNamespaceEntry` | 48 | `Act` | 在预期 revision 上替换 |
| `CompareAndRetireNamespaceEntry` | 49 | `Rollback` | 在预期 revision 上回收 |
| `ResolveNamespacePath` | 50 | 每跳 `Observe` | 解析一段或两段原生路径 |
| `ResolveNamespacePathFromMemory` | 51 | 每跳 `Observe` | 快照并解析三段或四段路径 |
| `CompareAndRebindNamespacePathFromMemory` | 52 | Mount `Observe` + 终端 `Act` | 原子比较并修改有界路径 |

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

快照 -> 解码 -> 逐跳鉴权 -> 比较 revision -> rebind
```

## 05 // 运行矩阵

| 子系统 | 原生路径 | QEMU 证据 |
| :--- | :--- | :--- |
| 隔离 | CR3 根、GDT/TSS/IDT、特权级切换 | `MULTI_AGENT_ISOLATION_OK` |
| 调度 | FIFO、PIT 抢占、CPU 帧恢复 | `MULTI_AGENT_CONTEXT_SWITCH_OK` |
| 故障 | `#UD`、`#GP`、`#PF`、路由、修复、重启 | `NATIVE_AGENT_FAULT_RESTART_OK` |
| IPC | 阻塞 Mailbox、唤醒、确认、回收 | `NATIVE_MAILBOX_IPC_OK` |
| 内存 | 页/区域分配、First-Fit 复用、清零 | `NATIVE_MEMORY_CONCURRENCY_OK` |
| 管理器 | Resource、Capability、Task、Agent、Memory、Namespace | `NATIVE_RESOURCE_MANAGER_AGENT_OK` |
| Admission | 常驻 Supervisor、Permit、批量释放 | `NATIVE_RUNTIME_ADMISSION_COMMIT_OK` |
| Driver | UART IRQ、HAL 请求、原生 Invocation | `DRIVER_INVOCATION_FLOW_OK` |
| 审计 | Event Log、SHA-256 归档、精确重放 | `NATIVE_EVENT_ARCHIVE_REPLAY_OK` |

## 06 // 验证档案

| 指标 | 值 |
| :--- | ---: |
| 目标 | `x86_64-unknown-none` |
| 隔离 Agent 上下文 | 11 |
| 内核选择 Dispatch | 35 |
| Resource Manager Calls / CR3 switches | `43 / 86` |
| Admission Supervisor Calls / CR3 switches | `44 / 88` |
| Namespace 容量 / 最终占用 | `4 / 4` |
| 实时 Event 容量 / 峰值 | `375 / 375` |
| 已归档 Event | 64 |
| 最终实时 Event / 下一序列 | `345 / 410` |
| 完整转录 | Events `1..409` |

| Native Capsule | Calls | 字节 | SHA-256 |
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
event[214..216]  namespace_entry_resolved  / Call 52 Mount
event[217]       namespace_entry_rebound   / revision 2 / Resource(3)
...
event[409]       driver_invocation_completed
SUPERVISOR_HANDOFF_READY
```

</details>

## 07 // 构建 + 启动

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

| 工具链组件 | 配置 |
| :--- | :--- |
| Rust | `rust-toolchain.toml` 固定 nightly |
| Target | `x86_64-unknown-none` |
| Image | BIOS 启动镜像 |
| Runtime | QEMU x86_64 |
| Binary audit | LLVM tools |

## 08 // 源码树

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

## 09 // 路线图

| 方向 | 当前 | 下一阶段 |
| :--- | :--- | :--- |
| Namespace | 类型化固定页消息；四跳修改 | 批量消息；委托修改 |
| Memory | 私有页表；页/区域复用 | 动态页表增长 |
| Scheduling | 单核 FIFO；PIT | SMP；同步；TLB shootdown |
| Durability | 有界 SHA-256 归档链 | 崩溃一致的签名存储 |
| Devices | UART；Port I/O | Storage；Network；Graphics；USB |
| Agent 软件 | 定宽 Capsule | Package 格式；生产加载器 |
| Assurance | 测试；QEMU 转录；ELF 审计 | 加固；形式化验证 |

`CURRENT_SPEC` [`Typed Namespace Path Rebind V5`](docs/superpowers/specs/2026-07-21-typed-namespace-path-rebind-v5-design.md)

## 10 // Patch 协议

| 门槛 | 要求 |
| :--- | :--- |
| `CONTRACT` | 阅读 [`AGENTS.md`](AGENTS.md) |
| `RED` | 运行时行为从失败测试开始 |
| `MODEL` | 保持显式权限与确定性 Event |
| `PROOF` | 提交聚焦测试与对应 QEMU 证据 |

```text
LICENSE  MIT
COPYRIGHT 2026 Ran
```
