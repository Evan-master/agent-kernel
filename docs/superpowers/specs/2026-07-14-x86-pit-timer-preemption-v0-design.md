# X86 PIT Timer Preemption V0 Design

## Purpose

The scheduler already accepts deterministic software ticks, but no physical
timer can drive that transition. This milestone connects one real x86 timer
interrupt to one running Agent Task:

```text
8254 channel 0 terminal count
  -> ISA IRQ0
  -> remapped 8259 PIC vector 0x20
  -> persistent IDT interrupt gate and assembly top half
  -> bounded architecture tick mailbox
  -> sys_tick_task(worker, task)
  -> TaskQuantumExpired
  -> accepted task returned to the fixed run queue
```

The architecture layer owns CPU, PIT, PIC, and IDT mechanics. The kernel owns
task authority, accounting, execution context, queue state, and the event log.
No kernel reference or capability is exposed to interrupt context.

## Hardware Contract

The 8254 channel 0 data port is `0x40` and its command port is `0x43`. V0 uses
channel 0, low-byte/high-byte access, binary Mode 3 (`0x36`), and divisor
`11_932`, which targets approximately 100 Hz from the canonical 1,193,182 Hz
input. The handler masks the master PIC before EOI, so this boot proof consumes
exactly one rising edge even though Mode 3 is periodic.

Intel documents the counter clock as 1.193 MHz, counter 0 as the IRQ0 system
timer, and Mode 3 as its typical operating mode. QEMU's in-tree i8254 model is
the emulator implementation used by the proof.

References:

- [Intel 700 Series PCH 8254 Timers](https://edc.intel.com/content/www/us/en/design/products-and-solutions/processors-and-chipsets/700-series-chipset-family-platform-controller-hub-datasheet-volume-1-of/002/8254-timers/)
- [QEMU 11.0.2 i8254 model](https://gitlab.com/qemu-project/qemu/-/blob/v11.0.2/hw/timer/i8254.c)

## Shared PIC Boundary

PIC initialization is moved out of the UART adapter into one architecture
module. It exposes only three operations needed during single-core boot:

- remap both 8259 controllers and expose one requested legacy IRQ,
- mask both controllers after a one-shot proof,
- provide fixed command/data ports and EOI values to assembly entries.

Each interrupt adapter still owns its source-specific arming and mailbox.
Initialization and mask changes happen with IF clear. V0 does not attempt a
dynamic interrupt registry or concurrent PIC policy.

## Persistent IDT Boundary

The exception runtime generalizes its UART-only gate installer to accept any
legacy IRQ vector at or above `0x20`. It rejects installation before the
persistent IDT is ready and rejects exception vectors. Callers guarantee IF is
clear; descriptors are published with volatile stores into the static table.

## Interrupt Top Half

The IRQ0 assembly entry saves the registers it uses, then:

1. increments a fixed byte counter,
2. masks all master-PIC lines to make the proof one-shot,
3. sends a non-specific EOI to the master PIC,
4. publishes the seen flag,
5. restores registers and returns with `iretq`.

It does not call Rust, allocate, print, invoke syscalls, inspect task state, or
hold kernel authority. The interrupted Rust context validates the count only
after reclaiming IF and masking both PICs.

## Agent Task Preparation

Normal kernel context registers Worker Agent 3 and declares one verified Act
intent over the bootstrap resource. It creates and delegates one task, then
registers and verifies a Worker image against that resource. The worker launches
through the delegated task capability, accepts the task, queues it, and
dispatches it with quantum `1`.

The resulting events are deterministic:

- events 16 through 24: worker registration, intent/task/delegation, image
  registration and admission,
- events 25 through 27: task acceptance, queueing, and dispatch.

## Bottom Half And Preemption

After exactly one physical tick is validated, normal kernel context calls
`sys_tick_task` for the running Worker task. Existing scheduler invariants
atomically:

- increment task `run_ticks` to `1`,
- decrement `quantum_remaining` to `0`,
- change task status from Running to Accepted,
- return the task to the fixed run queue,
- reset the Worker execution context to Idle,
- append event 28 as `TaskQuantumExpired`.

The later UART interrupt and causal Driver Invocation remain intact and occupy
events 29 through 38. The marker `AGENT_KERNEL_TIMER_PREEMPTION_OK` is printed
only after all postconditions are checked.

## Failure And Atomicity

A bounded spin prevents a missing IRQ from hanging QEMU. Missing, duplicate, or
premature timer signals fail before scheduler mutation. Worker admission or
dispatch failure aborts before the PIT is armed. Scheduler capacity and event
capacity checks remain those of `sys_tick_task`; a failure cannot partially
expire the task.

## Unsafe Boundary Audit

The release ELF exposes distinct `agent_kernel_pit_irq_stub` and
`agent_kernel_uart_irq_stub` symbols. Disassembly of the PIT entry confirms the
expected sequence: save `rax`/`rdx`, increment the fixed mailbox byte, write
`0xff` to PIC master data port `0x21`, write EOI `0x20` to command port `0x20`,
publish the seen byte, restore both registers, and execute `iretq`.

The optimized boot path also contains the exact PIT programming sequence:
command port `0x43` receives `0x36`, then channel 0 port `0x40` receives divisor
bytes `0x9c` and `0x2e`. The gate is installed through a volatile IDT descriptor
write while IF is clear. No assembly entry calls Rust or holds a pointer into
`AgentKernel`; the only semantic mutation occurs later through
`sys_tick_task`.

## Scope Limit

V0 proves physical timer ingress into the native scheduler, not an x86 user-mode
context switch. The interrupt returns to the boot context, which performs the
authorized bottom half. Saving full Agent CPU state, switching address spaces,
resuming another Agent, APIC timers, SMP routing, monotonic time, deadlines, and
sleep queues remain later milestones. Timer programming also remains trusted
boot authority in V0; exposing a kernel-owned Timer resource and capability is
required before ordinary Agents can configure it.
