# SMP Runtime V12 Design

Status: Frozen for implementation on 2026-07-23

## Objective

Replace the boot CPU assumptions with an explicit multiprocessor contract.
V12 discovers x86_64 processors from ACPI, starts application processors,
gives every online CPU private privilege and transition state, and requires a
generation-bound TLB shootdown before shared address-space mutations retire or
reuse physical frames.

The Agent execution model remains unchanged. Agents keep private address
spaces and capability-authorized kernel entry. SMP expands the machine that
executes those contracts without introducing POSIX process or thread models.

## Layer Ownership

| Layer | V12 responsibility |
| :--- | :--- |
| `agent-kernel-x86_64` library | CPU identifiers, fixed-capacity topology, CPU masks, shootdown protocol, ACPI and APIC contracts |
| `agent-kernel-x86_64` binary | Firmware mapping, BSP registration, AP startup, per-CPU tables and stacks, IPI execution, QEMU proof |
| `agent-kernel-core` | Existing deterministic Agent, Task, Capability, and Event state |
| `agent-kernel` | Existing Agent Call facade; no SMP-only public call |

Hardware coordination stays below the Agent Call ABI. CPU lifecycle evidence
is emitted by the architecture boot proof in V12; an Agent-visible hardware
topology resource is deferred until its capability and Event contract is
designed.

## Fixed Capacity

V12 supports at most 256 logical CPUs. This covers all xAPIC identifiers and
keeps masks and registries allocator-free. x2APIC identifiers remain 32-bit;
the registry maps each accepted firmware identifier to a dense `CpuIndex`.

| Type | Contract |
| :--- | :--- |
| `ApicId` | Firmware or hardware APIC identifier, full `u32` domain |
| `CpuIndex` | Dense kernel index in `0..256` |
| `CpuMask` | Four `u64` words, canonical bits only |
| `CpuTopology` | Insertion-ordered, duplicate-free descriptor array |
| `CpuRegistry` | Frozen topology plus explicit lifecycle state per CPU |

Firmware processors are admitted when MADT marks them enabled or online
capable. A descriptor records the ACPI processor UID, APIC ID, source entry
kind, and firmware flags. Duplicate APIC IDs, duplicate processor UIDs,
capacity overflow, an absent BSP, and a disabled BSP reject topology freeze.

Topology insertion order follows MADT entry order. The BSP always receives
logical index zero during freeze; remaining processors retain relative firmware
order. This produces stable CPU masks across replay and diagnostics.

## CPU Lifecycle

Each registered CPU follows one monotonic boot lifecycle:

~~~text
Discovered -> StartupRequested -> Online
                         |          |
                         v          v
                       Failed    Quiescing -> Offline
~~~

The BSP begins `Online`. An AP startup request carries a nonzero generation.
Only that CPU can acknowledge the matching generation. Repeated, stale, or
cross-CPU acknowledgements fail without mutation. A CPU may enter `Offline`
only after leaving all run ownership and TLB target masks.

Boot discovery and pre-Agent AP startup are intentionally outside the core
Event archive. Their serial proof binds topology, startup generations, and
online masks before `SUPERVISOR_HANDOFF_READY`.

## ACPI Topology Discovery

The bare-metal profile uses `acpi` with default features disabled. Its
allocator-free `AcpiTables` and raw MADT iterator validate firmware table
headers and checksums while the kernel owns the physical mapping policy.

The mapping handler translates physical ranges through the bootloader-provided
physical-memory offset and performs checked address and length arithmetic.
V12 accepts:

- Processor Local APIC entries;
- Processor Local x2APIC entries;
- I/O APIC entries;
- interrupt source overrides;
- a Local APIC address override.

At least one processor and one I/O APIC are required for the SMP QEMU profile.
Unknown MADT entries are skipped by the upstream iterator. Malformed tables,
ambiguous identities, unsupported SAPIC-only routing, and fixed-capacity
overflow stop boot with a precise marker.

## APIC Mode

V12 begins with xAPIC MMIO mode because QEMU exposes it consistently. The BSP
adds explicit supervisor RW, NX, uncacheable mappings for the Local APIC and
I/O APIC pages before starting any AP. CPUID and the IA32_APIC_BASE MSR must
report an enabled Local APIC. x2APIC topology may be discovered, but AP startup
fails explicitly when an identifier cannot be addressed by the active xAPIC
mode.

Legacy 8259 interrupts are masked after the I/O APIC routes are installed.
The PIT may remain the timer source for V12, routed through I/O APIC redirection
to the BSP. Local APIC vectors reserve distinct entries for timer, reschedule,
TLB shootdown, AP startup error, and spurious interrupts.

## Per-CPU Runtime

Every logical CPU owns a cache-line-aligned `PerCpuRuntime` slot containing:

- lifecycle and startup generation;
- kernel and current Agent CR3;
- host transition stack pointer;
- Agent Call, IRQ, and fault mailboxes;
- GDT, TSS, privileged-entry stack, and exception stack metadata;
- current Agent execution identity;
- observed shootdown generation.

The active slot is selected from the Local APIC ID after interrupts are
disabled. Assembly receives a pointer to that slot; no ring transition reads or
writes the former single global mailbox symbols. Per-CPU stacks have guard
pages and independent canaries.

## TLB Shootdown Contract

Page-table writers serialize through one architecture-owned mutation lock.
Before exposing a mapping change they create one `TlbShootdownRequest`:

| Field | Contract |
| :--- | :--- |
| generation | Monotonic, nonzero, never reused |
| address space | CR3 root plus kernel-owned address-space generation |
| scope | one page, bounded range, whole address space, or all contexts |
| initiator | Online CPU that performed its local invalidation |
| targets | Snapshot of online CPUs that may cache the address space |
| acknowledged | Initially empty and always a subset of targets |

The initiator is excluded from targets after executing its local invalidation.
Targets receive the dedicated IPI, validate the request generation, execute
`invlpg` or a CR3 reload for the declared scope, then publish acknowledgement
with Release ordering. The initiator observes acknowledgements with Acquire
ordering.

Only one request is active per coordinator. Completion requires exact target
coverage. Stale generations, non-target acknowledgements, duplicates, invalid
ranges, offline initiators, and a second concurrent begin operation fail
without changing the active request. Generation overflow is terminal.

Physical frames removed from any address space remain quarantined until the
matching request completes. Address-space generation prevents a stale IPI from
flushing a newly constructed address space that reuses the same CR3 frame.

## Synchronization

V12 provides an IRQ-safe ticket lock for short architecture state mutations.
Lock acquisition disables local interrupts and records the previous IF state;
release publishes writes before restoring that state. Recursive acquisition is
an error in debug contracts and unsupported in bare-metal execution.

The run queue remains deterministic FIFO. V12 initially gives the BSP sole
ownership of kernel scheduling decisions while APs execute dispatched native
contexts. A single scheduler lock protects ownership transfer. Work stealing,
NUMA placement, and scheduler affinity are deferred.

## AP Startup

The BSP reserves one low-memory trampoline page below 1 MiB and a bounded AP
handoff block. For each AP, serially:

1. publish target APIC ID, logical index, CR3, stack, and startup generation;
2. send INIT, wait the architectural interval, send two SIPIs;
3. let 16-bit trampoline code enter long mode using the kernel page tables;
4. resolve and initialize the target `PerCpuRuntime`;
5. load that CPU's GDT, TSS, IDT, and Local APIC state;
6. acknowledge the matching generation and enter the kernel idle loop.

The BSP waits with a bounded PIT-derived deadline. A failed AP remains recorded
as `Failed`; it never enters online or shootdown target masks.

## Native Proof

The QEMU SMP profile boots with at least two virtual CPUs and proves:

- ACPI topology contains distinct BSP and AP identifiers;
- AP startup reaches an online mask with at least two bits;
- each CPU reports a distinct privileged stack and per-CPU slot;
- one Agent context executes on an AP and crosses Ring 3 to Ring 0;
- the BSP mutates an address-space mapping cached by the AP;
- the AP handles the shootdown IPI and acknowledges the exact generation;
- stale and duplicate acknowledgements do not complete another request;
- frame reuse happens only after exact target acknowledgement;
- existing V11 trust rotation and deterministic Event evidence remain intact.

Successful completion emits:

- `AGENT_KERNEL_ACPI_TOPOLOGY_OK`;
- `AGENT_KERNEL_PER_CPU_GUARD_PAGES_OK`;
- `AGENT_KERNEL_SMP_AP_ONLINE_OK`;
- `AGENT_KERNEL_PER_CPU_PRIVILEGE_OK`;
- `AGENT_KERNEL_AP_AGENT_CALL_OK`;
- `AGENT_KERNEL_TLB_SHOOTDOWN_OK`;
- `AGENT_KERNEL_TLB_FRAME_REUSE_OK`;
- `AGENT_KERNEL_SMP_HANDOFF_READY`.

## Verification Gates

- CPU masks test boundaries at indices 0, 63, 64, 255 and reject 256.
- Topology tests cover ordering, BSP remap, xAPIC and x2APIC entries,
  duplicate IDs and UIDs, disabled entries, missing BSP, and capacity.
- Lifecycle tests cover every legal edge and prove stale operations are atomic.
- Shootdown tests cover scopes, target snapshots, exact completion, stale and
  duplicate acknowledgement, empty targets, overlap, and generation overflow.
- ACPI fixture tests cover RSDP v1/v2, RSDT/XSDT, checksums, MADT variants,
  malformed lengths, unknown entries, overrides, and capacity.
- Synchronization tests validate ticket order and publication on host threads.
- Workspace, Supervisor, freestanding, strict Clippy, debug QEMU `-smp 2`,
  Release QEMU `-smp 2`, assembly, ELF, and formatting gates pass.

## Deferred Work

- x2APIC MSR execution mode and more than 256 CPUs;
- NUMA topology and affinity-aware scheduling;
- concurrent shootdowns for independent address spaces;
- work stealing and per-CPU run queues;
- CPU hot-add and physical hot-remove;
- hardware IOMMU invalidation;
- power-state and frequency management.
