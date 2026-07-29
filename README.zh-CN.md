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
[05] native state signer .... TPM-bound
[06] PCI device fabric ...... driving native I/O
[07] DMA authority .......... VT-d enforced
[08] message interrupts ..... MSI/MSI-X active
[09] native network ......... ARP round-trip
kernel://network/v29-virtio-net
</pre>

</div>

```text
┌─ SYSTEM STATUS ─────────────────────────────────────────────────┐
│ VERIFIED   V29 / QEMU debug + release   virtio-net + VT-d       │
│ KERNEL     no_std / 无堆                 ISA    x86_64           │
│ MODE       ring 0 + ring 3              ABI    Agent Call       │
│ STATE      ATA A/B + 原生以太网           AUTH   Capability       │
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
| `agent-kernel-x86_64` | 启动、分页、特权切换、IRQ、PCI、MSI/MSI-X、virtio-rng、virtio-net、ATA PIO、TPM CRB、DMAR、VT-d、原生执行 |
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
| I/O | Capability 授权的 HAL 请求、原生以太网、INTx/MSI/MSI-X、PCI BAR 认领、共享 VT-d DMA、virtio-rng、virtio-net、端口与 ATA PIO |

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
                   sign(56) ──> 内核 TPM service ──> CRB
                                            │
                                            ▼
commit(55) <── 精确 384B request <── 低 S P-256 signature
```

| 契约 | V13 至 V27 不变量 |
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
| TPM 权限 | Ring 0 独占 MMIO 与命令传输；Ring 3 仅可请求一次保留 Manifest 签名 |
| Event 历史 | Disabled 保留授权只读快照；持久提交仅释放已验证前缀 |

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
| Provider ABI | Manifest、Signature、Policy Generation、已认证 Agent/Task/Image |
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

```text
V19 NATIVE TPM STATE SIGNER
discovery         ACPI TPM2 / Start Method 7
transport         CRB locality 0 / 有界轮询 / 清理失败即禁用
binding           ReadPublic Name + Template + 压缩 P-256 Point
agent boundary    Call 56 / 仅保留 Manifest / 无原始 TPM 通道
recovery proof    TPM 签名 / ATA 提交 / 断电 / 冷启动恢复
```

```text
V20 TPM MEASURED POLICY
policy            SHA-256 PCR 位图 + 预期复合摘要
authorization     PolicyPCR -> PolicyCommandCode -> Sign
template          精确 authPolicy / 关闭 userWithAuth / 启用 adminWithPolicy
lifecycle         新建会话 / 显式 FlushContext / 故障后禁用
recovery proof    PCR 策略 / TPM 签名 / ATA 提交 / 冷启动恢复
```

```text
V21 NATIVE PCI INVENTORY
transport         Configuration Mechanism 1 / 0x0cf8 + 0x0cfc
coordinates       segment 0 / 256 Bus / 32 Device / 8 Function
inventory         256 条固定记录 / BDF 稳定排序 / 只读
classes           Network / Display / USB 发现
ownership         BSP 持有 / Agent 原始配置访问关闭
```

```text
V22 PCI RESOURCE CLAIMS
probe             关闭 Decode / 全一值测量 / 精确恢复
catalog           稳定 BDF / Type 0 Endpoint / 已分配 BAR
tree              Function Resource + 1..6 个 BAR Region Resource
authority         每节点 Owner Capability / 每 BAR Physical Endpoint
transaction       完整预检 / 有序 Event / 原子提交
agent surface     ResourceId + Capability / 关闭原始配置修改
```

```text
V23 CAPABILITY-BOUND PCI DRIVER
target            0000:00:04.0 / 1b36:0002 / BAR0 I/O 8B
admission         退休 Worker 入口 / 回收镜像槽 / Driver 镜像
authority         BAR Capability / Observe + Act / Driver Binding
execution         StateChanged / Invocation / 不可变 Write Command
backend           有界 16550 THRE 轮询 / 原生 x86 OUT
physical proof    File Chardev / 唯一字节 / 0x50
executor          Ring-0 基线 / V24 原生执行路径接管
```

```text
V24 NATIVE DRIVER AGENT CALL
image             Capsule v1 / kind 6 / 323 字节 / 单 RX 页
calls             Describe / Inspect / Acknowledge / Submit / Complete
context           Agent + DriverInvocation + Image + Capability
scheduler         Core FIFO / Local APIC 抢占 / 完整 Frame 恢复
authority         Ring 3 使用语义 ID / Ring 0 持有 Endpoint 与 Port
proof             五次 Call Transcript / 单次 0x50 OUT / 完整帧回收
```

```text
V25 PCI INTX + DRIVER RESTART
route             IRQ 11 / INTA / 低电平 Level / Vector 0x2b
ingress           单次 IIR + LSR 捕获 / 关闭源 / Mask + EOI
state flow        #UD / Core Faulted / Owner Rollback / Generation 1
commands          Configure(ARM_THRE_INTERRUPT) / Write(0x50)
proof             两次 Invocation / 真实 INTx / 两次地址空间回收
```

```text
V26 QEMU ATA POWER-LOSS
writer            Signed Package v3 / 复用 Agent 11 / Calls 54,56,55
TPM               swtpm CRB / PCR 23 / TPM2_CC_Sign / P-256
storage           ATA Primary Slave / A/B 槽 / Generation 1 / Event 1..64
cut               宿主在持久提交标记后执行 SIGKILL
recovery          无 TPM / Event 65..516 / 磁盘不变 / PCI 字节 0x50
```

```text
V27 NATIVE DMA/IOMMU AUTHORITY
target            Q35 / 0000:00:05.0 / QEMU EDU 1234:11e8
firmware          校验有效的 DMAR / 精确 DRHD Requester Scope
authority         IOMMU + Device + Memory + DMA Domain Capability
translation       Intel VT-d Root/Context/三级 Second-Level 页表
proof             双向 DMA / 撤销 / 写 Fault / RAM 保持不变
```

`TPM CRB` 完成 · `PCI NATIVE I/O` 完成 · `RING-3 DRIVER` 完成 · `ATA POWER-LOSS` 完成 · `VT-d DMA` 完成

## `05 // AGENT CALL`

```text
┌─ REGISTER FRAME ────────────────────────────────────────────────┐
│ rax magic    rbx ABI       rcx operation / status              │
│ rsi Agent    rdi Task/Invocation    r8 Image    r9 Nonce       │
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
| `54..56` | 持久归档 Prepare、TPM 签名与签名 Commit |
| `57..60` | Driver Invocation 检查、Event 确认、命令提交、完成 |

`TRANSPORT` 私有 call-data 页 · `POINTERS` 拒绝 · `REPLY` 规范寄存器

## `06 // 启动证据`

```text
PROFILE A          V26 qemu-ata-power-loss
PROFILE B          V27 qemu-dma-iommu
PROFILE C          V28 qemu-msi-msix
PROFILE D          V29 qemu-native-net
QEMU               全部 Profile / debug + release
BASELINE EVENTS     1..451 / 精确 V25 历史
DURABLE HEAD        Generation 1 / Event 1..64
RECOVERY EVENTS     65..516 / 有序连续历史
AGENT ENTRIES       12 个活跃入口 / 回收槽
TASK DISPATCHES     35
V25 DRIVER RUNS     6 / 三次 Quantum Expiry / 一次重启
V27 REQUESTER       0000:00:05.0 / Source 0x28
V27 MAPPING         IOVA 0x01000000 / 单个 4 KiB 页
V27 REVOCATION      VT-d Reason 5 / 写入阻断 / RAM 不变
V28 REQUESTERS      EDU 00:05.0 + virtio-rng 00:06.0 / Domain 1
V28 ROUTES          MSI 0xd0 + MSI-X[0] 0xd1
V28 DETACH          Source 0x30 被拒绝 / EDU 保持运行
V29 ENDPOINT         52:54:00:12:34:56 / MTU 1500
V29 QUEUES           RX 0 / TX 1 / 每队列一个 Descriptor
V29 TRAFFIC          ARP 请求 + 网关回复 / MSI-X 0xd2 + 0xd3
V29 DENIAL           已脱离 Source 0x28 / VT-d Fault / 无完成中断
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
| Event 历史 | `AGENT_KERNEL_NATIVE_EVENT_SNAPSHOT_HISTORY_OK` |
| PCI Function 认领 | `AGENT_KERNEL_PCI_FUNCTION_CLAIM_OK` |
| PCI 权限边界 | `AGENT_KERNEL_PCI_CAPABILITY_BOUNDARY_OK` |
| PCI Driver 准入 | `AGENT_KERNEL_PCI_SERIAL_AGENT_REUSED_OK` |
| Driver 镜像 | `AGENT_KERNEL_PCI_SERIAL_DRIVER_IMAGE_OK` |
| PCI INTx 路由 | `AGENT_KERNEL_PCI_INTX_ROUTE_OK` |
| Driver Fault 隔离 | `AGENT_KERNEL_PCI_SERIAL_DRIVER_FAULT_CONTAINED_OK` |
| Driver 重启 | `AGENT_KERNEL_PCI_SERIAL_DRIVER_RESTARTED_OK` |
| 物理 PCI INTx | `AGENT_KERNEL_PCI_SERIAL_INTX_OK` |
| Ring-3 Driver | `AGENT_KERNEL_PCI_SERIAL_RING3_DRIVER_OK` |
| PCI 物理命令 | `AGENT_KERNEL_PCI_SERIAL_PHYSICAL_IO_OK` |
| Driver 地址空间回收 | `AGENT_KERNEL_PCI_SERIAL_ADDRESS_SPACE_RECLAIMED_OK` |
| PCI 终态 | `AGENT_KERNEL_PCI_SERIAL_DRIVER_OK` |
| 持久 Writer 提交 | `AGENT_KERNEL_QEMU_DURABLE_COMMIT_OK` |
| 强制断电恢复 | `AGENT_KERNEL_QEMU_DURABLE_POWER_LOSS_OK` |
| DMAR 发现 | `AGENT_KERNEL_DMAR_DISCOVERY_OK` |
| PCI Bus Master 门控 | `AGENT_KERNEL_DMA_BUS_MASTER_QUIESCED_OK` |
| DMA Capability | `AGENT_KERNEL_DMA_CAPABILITY_OK` |
| VT-d Translation | `AGENT_KERNEL_VTD_TRANSLATION_OK` |
| 已授权 DMA | `AGENT_KERNEL_DMA_ALLOWED_OK` |
| 已撤销 DMA Fault | `AGENT_KERNEL_DMA_REVOKED_FAULT_OK` |
| DMA/IOMMU 终态证明 | `AGENT_KERNEL_DMA_IOMMU_PROOF_OK` |
| Interrupt Route Capability | `AGENT_KERNEL_INTERRUPT_CAPABILITY_OK` |
| 共享 DMA Domain | `AGENT_KERNEL_MULTI_DEVICE_DMA_DOMAIN_OK` |
| EDU MSI 配置 | `AGENT_KERNEL_MSI_CONFIGURED_OK` |
| EDU MSI 送达 | `AGENT_KERNEL_EDU_MSI_DELIVERED_OK` |
| virtio-rng MSI-X 配置 | `AGENT_KERNEL_MSIX_CONFIGURED_OK` |
| virtio-rng MSI-X 送达 | `AGENT_KERNEL_VIRTIO_RNG_MSIX_DELIVERED_OK` |
| Requester 脱离 | `AGENT_KERNEL_DMA_REQUESTER_DETACHED_OK` |
| 已脱离 Requester Fault | `AGENT_KERNEL_DMA_DETACH_FAULT_OK` |
| 共享 Domain 存活设备 | `AGENT_KERNEL_SHARED_DOMAIN_SURVIVOR_OK` |
| MSI/MSI-X 终态证明 | `AGENT_KERNEL_MSI_MSIX_PROOF_OK` |
| Network Capability | `AGENT_KERNEL_NATIVE_NET_CAPABILITY_OK` |
| Network DMA Domain | `AGENT_KERNEL_NATIVE_NET_DMA_DOMAIN_OK` |
| virtio-net TX MSI-X | `AGENT_KERNEL_NATIVE_NET_TX_MSIX_DELIVERED_OK` |
| ARP 往返 | `AGENT_KERNEL_NATIVE_NET_ARP_REPLY_OK` |
| 已脱离 Network DMA | `AGENT_KERNEL_NATIVE_NET_DMA_DENIAL_OK` |
| 原生网络终态证明 | `AGENT_KERNEL_NATIVE_NET_PROOF_OK` |
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

```text
V19 NATIVE TPM
ACPI               校验和有效的 TPM2 / Start Method 7
CRB                Locality + Ready + Execute + Cleanup
wire               ReadPublic / SignDigest v185 / Sign v184
key binding        Name + TPMT_PUBLIC + P-256 Point
Agent Call         56 / 仅 Generation Payload
closed loop        TPM Response / ATA 提交 / 冷启动恢复
```

```text
V20 TPM POLICY
PCR                单 SHA-256 Bank / PCR 0..23 位图
digest             PolicyPCR + PolicyCommandCode
key binding        精确 authPolicy / 拒绝密码旁路
session            创建 / 断言 / 签名 / 清理
closed loop        Policy Session / ATA 提交 / 断电 / 冷启动恢复
```

```text
V21 PCI INVENTORY
selector           已验证 BDF + 对齐公共 Header DWORD
probe              保存 / 验证 / 恢复 Address Latch
scan               缺席 / 单 Function / 多 Function / 全 Bus
failure            无 Function / 容量溢出 -> 停止启动
boot evidence      PCI_CONFIG_IO_OK / PCI_INVENTORY_OK
```

```text
V22 PCI AUTHORITY
BAR probe           I/O + 32-bit + below-1-MiB + 64-bit
hardware safety     关闭 Command Decode / 恢复 BAR
Core transaction    Resource + Capability + Driver Endpoint
claim mapping       每个 BAR Slot 绑定精确内核权限
QEMU suffix         Event 413..417 / 一个已分配 BAR
boot evidence       BAR_CATALOG_OK / FUNCTION_CLAIM_OK / CAPABILITY_BOUNDARY_OK
```

```text
V24 NATIVE PCI SERIAL DRIVER
selection           精确 BDF + Vendor + Device / 必须存在可认领 BAR
lifecycle           退休 Pending Image / 删除记录 / 复用槽位
admission           Agent 10 重新启动为 BAR Scope Driver
ring-3 calls         1,57,58,59,60 / Offset 46,85,182,220,289
scheduling          Dispatch / Quantum Expiry / Frame 恢复 / Completion
request              Write / Opcode 0 / Value 0x50 / 不可变因果链
hardware            有界 LSR 轮询 / 单次 x86 OUT / 拒绝路径零 I/O
reclamation         SMP TLB Shootdown / 精确帧清零并归还
QEMU suffix         Event 418..435 / 精确 0x50 Chardev 字节
```

```text
V25 NATIVE PCI INTX DRIVER
capsule             437 字节 / Fault Offset 187 / Generation Signal
route               INTA / IRQ 11 / 低电平 Level / 初始 Mask
state Invocation    Event 425..440 / #UD / Fault + Recovery / Configure
interrupt           Event 441..451 / 硬件 IIR 0x02 / LSR THRE
recovery            Rollback Capability / 单代 / 零 Command Evidence
hardware            Configure IER 0x02 / 真实 INTx / 单次 Write(0x50)
reclamation         两次 Invocation 分别执行 SMP TLB Shootdown
```

```text
V26 QEMU ATA POWER-LOSS
writer              Ring-3 State Signer / Agent 11 / Event 1..64
signing             TPM CRB / PCR Policy / 低 S P-256
media               Raw ATA Slave / Slot A Committed / Generation 1
power cut           宿主 SIGKILL / 不经过 Guest Shutdown
inspection          Canonical Manifest / Payload Digest / Signature / Chain
recovery            无 TPM 设备 / 首条 Event 65 / 终态 Event 516
immutability        恢复前后持久镜像 SHA-256 相同
```

```text
V27 NATIVE DMA/IOMMU
discovery           ACPI DMAR / DRHD / QEMU EDU BAR0
gate                配置阶段关闭 PCI Memory + Bus Master
Core                创建 Domain / 绑定 Requester / Reserve / Activate
allowed             RAM -> EDU -> RAM / 精确 Pattern 恢复
revoke              清除 Leaf / Context + IOTLB Invalidate / Release
blocked             设备写 Fault / Source 0x28 / 目标页不变
```

```text
V28 NATIVE MSI/MSI-X
routes              Core Resource / MSI 0xd0 / MSI-X Entry 0 at 0xd1
domain              Domain 1 / EDU 00:05.0 + virtio-rng 00:06.0
mappings            EDU Data + Split Queue + Entropy / 三个 4 KiB Leaf
delivery            原生 IDT Handler / 设备 Cause Ack / Local APIC EOI
detach              移除 Requester 0x30 / Context + IOTLB Invalidation
isolation           virtio DMA 被拒绝 / Entropy Sentinel 不变
survivor            virtio 脱离后 EDU DMA + MSI 继续完成
```

```text
V29 NATIVE VIRTIO NETWORK
authority           Network Resource / Act + Observe + Rollback
identity            MAC 52:54:00:12:34:56 / MTU 1500
queues              RX 0 + TX 1 / 单 Descriptor / 四个 DMA 页
delivery            MSI-X Vector 0xd2 + 0xd3 / Local APIC EOI
traffic             ARP 请求 10.0.2.15 -> 网关 10.0.2.2
evidence            SHA-256 Frame Descriptor / 有序 Core Event
isolation           Requester 脱离 / VT-d Fault / 零完成中断
```

<details>
<summary><code>已验证镜像清单</code></summary>

| 原生镜像 | 格式 | Calls | 字节 | SHA-256 |
| :--- | :--- | ---: | ---: | :--- |
| Resource Manager | Signed Package v3 | 44 | 17,093 | `4500e02b07cb...43d18745` |
| Admission Supervisor | Capsule v1 | 44 | 4,122 | `54eaa321a65c...923e10970` |
| PCI Serial Driver | Capsule v1 / kind 6 | 5 | 437 | `95787586c02f...eec2e402` |

`AUDIT` 9 个原生镜像 · 2 个签名 Package v3 镜像 · 5 个精确 Assembly 源

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
$ scripts/run-qemu-dma-iommu.sh
$ scripts/run-qemu-dma-iommu.sh --release
$ scripts/run-qemu-msi-msix.sh
$ scripts/run-qemu-msi-msix.sh --release
$ scripts/run-qemu-native-net.sh
$ scripts/run-qemu-native-net.sh --release
$ ruby scripts/audit-agent-images.rb --assembly
$ ruby scripts/test-state-signer-package.rb
$ ruby scripts/test-inspect-tpm-state-signer.rb
$ ruby scripts/test-inspect-qemu-durable-disk.rb
$ ruby scripts/test-qemu-durable-profile.rb
```

```console
$ ruby scripts/run-qemu-durable-power-loss.rb
$ ruby scripts/run-qemu-durable-power-loss.rb --release
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
    --policy-generation 1 --authorization pcr-policy \
    --pcr-selection "$TPM_PCR_SELECTION" \
    --pcr-digest "$TPM_PCR_DIGEST" \
    --name "$TPM_NAME" --public-key "$TPM_PUBLIC_KEY"
```

硬件 Profile 默认使用 `Disabled`。启用路径采用
`NativeTpmSignerProfile::Crb`、检查器计算的 `authPolicy` 与匹配的 ATA
Signer Record。

```console
$ cargo check -p agent-kernel-x86_64 \
    --features bare-metal \
    --bin agent-kernel-x86_64 \
    --target x86_64-unknown-none
```

`TOOLCHAIN` Rust nightly · `EMULATOR` QEMU x86_64 + swtpm + Intel VT-d + EDU + virtio-rng + virtio-net · `PROVISIONER` Go · `TARGET` x86_64-unknown-none

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
scripts/{run-qemu.sh,run-qemu-dma-iommu.sh,run-qemu-msi-msix.sh,run-qemu-native-net.sh,run-qemu-durable-power-loss.rb}
tools/qemu-tpm-provision/
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
[done] ACPI TPM2 发现 + CRB 传输 + Provisioned Signer 绑定
[done] Agent Call 56 + 内置 TPM Provider + 脚本化 TPM 恢复证明
[done] SHA-256 PCR Policy Session + 命令绑定 TPM 授权
[done] 策略门控 TPM 签名 + ATA 断电恢复证明
[done] 原生 PCI 配置访问 + 固定容量 Function 清单
[done] 可逆 PCI BAR Probe + Capability 绑定的 Driver Function 认领
[done] 精确 PCI Serial 选择 + Capability 绑定的物理命令
[done] 原生 Ring-3 Driver Agent Call + 可抢占 PCI Serial 执行
[done] PCI INTx 路由 + Driver Fault 隔离与重启
[done] QEMU 独立 ATA 镜像 + SIGKILL 断电恢复证明
[done] Capability 绑定 DMA Domain + Intel VT-d 授权/撤销证明
[done] MSI/MSI-X Interrupt Route + 共享多设备 DMA Domain
[done] 现代 virtio-rng + Requester 级 VT-d 脱离证明
[done] Capability 治理的 Network Endpoint + 现代 virtio-net ARP 证明
[next] IPv4/UDP + Graphics + USB Controller + 形式化验证
```

| 轨道 | 记录 |
| :--- | :--- |
| 已验证基线 | [Signed Agent Package V10](docs/superpowers/specs/2026-07-21-signed-agent-package-v10-design.md) |
| Runtime 里程碑 | [SMP Runtime V12](docs/superpowers/specs/2026-07-23-smp-runtime-v12-design.md) |
| 持久协议 | [Signed Durable State V13](docs/superpowers/specs/2026-07-23-signed-durable-state-v13-design.md) |
| 原生存储 | [Native ATA Durable State V14](docs/superpowers/specs/2026-07-23-native-ata-durable-state-v14-design.md) |
| PCI 发现 | [Native PCI Inventory V21](docs/superpowers/specs/2026-07-27-native-pci-inventory-v21-design.md) |
| PCI 权限 | [PCI Resource Claims V22](docs/superpowers/specs/2026-07-27-pci-resource-claims-v22-design.md) |
| PCI 设备路径 | [Native PCI Serial Driver V23](docs/superpowers/specs/2026-07-27-native-pci-serial-driver-v23-design.md) |
| 持久化里程碑 | [QEMU ATA Power-Loss V26](docs/superpowers/specs/2026-07-28-qemu-ata-power-loss-v26-design.md) |
| DMA 基础 | [Native DMA/IOMMU V27](docs/superpowers/specs/2026-07-28-native-dma-iommu-v27-design.md) |
| 当前里程碑 | [Native Virtio Network V29](docs/superpowers/specs/2026-07-29-native-virtio-net-v29-design.md) |

## `10 // 项目`

| 字段 | 值 |
| :--- | :--- |
| 工程契约 | [`AGENTS.md`](AGENTS.md) |
| 许可证 | [`MIT`](LICENSE) |
| 状态 | 持续开发 |

```text
AGENT KERNEL // CONTROL PLANE FOR AUTONOMOUS MACHINES // 2026
```
