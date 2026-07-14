# X86 Agent CPU Context V0 Design

## Purpose

PIT Timer Preemption V0 proves that a physical IRQ0 can drive the semantic
scheduler, but the interrupted CPU is still executing the boot coordinator.
This milestone gives one admitted Worker task an actual x86_64 execution
context and connects its hardware frame to the kernel lifecycle:

```text
TaskDispatched
  -> fixed Agent stack + native entry RIP
  -> context switch from kernel stack to Agent stack
  -> Agent arms PIT and executes native instructions
  -> IRQ0 saves all integer registers + RIP/CS/RFLAGS on Agent stack
  -> IRQ top half restores the saved kernel stack
  -> TaskQuantumExpired and requeue
  -> TaskDispatched again
  -> restore interrupt frame with iretq
  -> Agent resumes at the exact interrupted RIP and yields
  -> TaskYielded and requeue
```

The Agent remains ring 0 in V0. This milestone establishes real CPU context
ownership and asynchronous preemption before adding privilege separation,
address-space isolation, or untrusted executable loading.

## ABI And Target Contract

The context switch follows the x86-64 System V ABI. A normal switch preserves
`rbx`, `rbp`, and `r12` through `r15`, stores the outgoing `rsp`, loads the
incoming `rsp`, restores those registers, and returns into the incoming RIP.
The bootstrap frame reserves 64 bytes so the new Rust entry observes the ABI
requirement `rsp % 16 == 8` after the assembly `ret`.

The bare-metal target contract is also part of the safety proof. Rust's
`x86_64-unknown-none` target disables the red zone and all MMX/SSE/AVX features,
uses soft float, and supports 64-bit atomics. Therefore an interrupt may use the
current Agent stack without corrupting an ABI red zone, and V0 does not need an
XSAVE area to preserve compiler-generated vector state.

References:

- [Intel 64 and IA-32 Software Developer Manuals](https://www.intel.com/content/www/us/en/developer/articles/technical/intel-sdm.html)
- [x86-64 System V psABI](https://gitlab.com/x86-psABIs/x86-64-ABI)

## Portable Layout Contract

The architecture library defines two `repr(C)` frame layouts so host tests can
lock every assembly offset:

- `CalleeSavedFrame`: six ABI-preserved registers plus return RIP, 56 bytes,
- `InterruptStackFrame`: 15 integer registers followed by hardware-pushed RIP,
  CS, and RFLAGS, 144 bytes.

The IRQ handler pushes registers in the reverse order represented in memory,
making `rip` reside at byte offset 120. These layouts describe same-privilege
interrupts only; no SS/RSP pair is present because V0 does not cross rings.

## Fixed Agent Runtime

The architecture binary owns one 32 KiB, 16-byte-aligned static Agent stack and
three fixed context slots: saved kernel `rsp`, cooperative Agent `rsp`, and the
preempted interrupt-frame `rsp`. Access is restricted to the single-core boot
proof with IF clear except while the Agent is deliberately running.

The runtime is exposed as type states:

- `PreparedAgentCpu` can enter exactly once,
- `PreemptedAgentCpu` proves a validated IRQ frame exists,
- `YieldedAgentCpu` proves that frame was resumed and the Agent switched back.

No public constructor exists for the preemption or yield tokens. The semantic
scheduler consumes them only in the corresponding task transition.

## Initial Context Switch

The kernel calls the context-switch assembly with IF clear. The new Agent stack
returns into a fixed Rust entry function. That function records its stack
address, arms PIT IRQ0 while still non-interruptible, executes `sti`, and spins
on an architecture-local preemption flag. Arming from the Agent stack removes
the race where an already-pending IRQ could arrive while the kernel stack was
still active.

## IRQ0 Preemption

The interrupt gate clears IF and the top half pushes all 15 general-purpose
registers. It records the frame `rsp` and hardware-pushed RIP, increments the
one-shot tick count, masks the master PIC, sends EOI, and publishes the
preemption flag. Instead of `iretq`, it loads the previously saved kernel
context, restores its ABI-preserved registers, and returns to the coordinator.

The complete Agent frame remains untouched on the Agent stack. The kernel
validates that the frame lies inside the fixed stack and that exactly one tick
occurred before calling `sys_tick_task`.

## Scheduler And Resume Ordering

The first semantic tick produces event 28 `TaskQuantumExpired`, requeues the
task, and makes the Worker idle. The kernel then dispatches the same task again
with quantum one, producing event 29. Only after this event may the architecture
runtime resume the saved CPU frame.

The resume assembly saves a fresh kernel continuation, loads the interrupt
frame, restores all integer registers, and executes `iretq`. The Agent resumes
at the captured RIP with its original RFLAGS. It observes the preemption flag,
records a resume marker, and performs a normal cooperative context switch back
to the saved kernel continuation. An assembly trampoline executes `cli` before
returning to Rust. The kernel then calls `sys_yield_task`, producing event 30 and
returning the task to Accepted/queued/Idle state.

The existing UART interrupt and Driver Invocation remain events 31 through 40.

## Failure And Authority

Stack-layout, alignment, initialization, frame-range, tick-count, task-state,
resume, or yield validation failure exits QEMU before claiming handoff success.
The IRQ top half never calls Rust, allocates, prints, or mutates kernel stores.
Kernel mutations still pass through capability-checked public syscalls and each
successful transition has a replayable event.

The architecture stack and frame markers are intentionally not separate kernel
events: they are implementation evidence for the already-recorded dispatch,
quantum-expiry, redispatch, and yield transitions, not independent Agent
operations.

## Unsafe Boundary Audit

The release ELF was inspected after the final build. Its three assembly
boundaries were present as standalone symbols:

- agent_kernel_context_switch at address 0xe062,
- agent_kernel_resume_interrupted_agent at address 0xe07d,
- agent_kernel_agent_timer_irq_stub at address 0xe0b0.

The normal switch pushes and pops exactly rbp, rbx, and r12 through r15 around
the outgoing/incoming stack-pointer exchange. The interrupt top half pushes all
15 integer registers, reads RIP from offset 0x78, masks the master PIC through
port 0x21, sends EOI through port 0x20, and loads the saved kernel stack before
returning. It makes no Rust call while the interrupt frame is active.

The resume path restores those 15 registers in the inverse order and executes
iretq. Its saved-host trampoline executes cli before returning to Rust, so all
storage validation again occurs with IF clear. The remaining unsafe Rust
accesses are volatile reads and writes within the single fixed stack and
context slots; their exclusivity depends on the documented single-core
type-state ownership rule.

## Scope Limit

V0 stores integer state only because the selected target forbids hardware
floating/vector code. It does not provide ring 3, TSS privilege-stack switching,
per-Agent page tables, guard pages around the static Agent stack, multiple CPU
contexts, context migration, SMP, XSAVE, debug registers, TLS bases, or
executable image loading. Those remain explicit follow-up milestones.
