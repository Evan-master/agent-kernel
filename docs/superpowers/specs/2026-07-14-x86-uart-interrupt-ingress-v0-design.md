# X86 UART Interrupt Ingress V0 Design

## Purpose

The current QEMU path polls COM1 line status through a kernel command and then
turns the result into a Device Event. That proves physical I/O, but the event is
still synchronously manufactured by boot orchestration.

This milestone replaces that polling source with a real x86 hardware interrupt:

```text
16550 THRE condition
  -> ISA IRQ4
  -> remapped 8259 PIC vector 0x24
  -> x86 IDT interrupt gate and assembly top half
  -> bounded architecture interrupt mailbox
  -> kernel DeviceEvent::Interrupt
  -> Driver Invocation
  -> causal COM1 write command
```

The event, invocation, command, capability, and execution-context semantics stay
kernel-owned. The architecture layer owns only CPU/PIC/UART mechanics and one
minimal handoff record.

## Verified Emulator Semantics

QEMU's 16550 implementation defines IER bit `0x02` and IIR identity `0x02` as
the transmitter-holding-register-empty interrupt. When that source is pending,
QEMU raises the UART IRQ. Reading IIR clears the pending THRE source and updates
the IRQ line. The V0 handler also disables UART interrupts before acknowledging
the PIC, making the one-shot proof explicit.

Reference: [`hw/char/serial.c`](https://gitlab.com/qemu-project/qemu/-/blob/v11.0.2/hw/char/serial.c)
in the QEMU source tree.

## IDT Contract

`agent-kernel-x86_64` gains portable fixed-width IDT encoding primitives:

- a 16-byte long-mode IDT entry,
- a 10-byte IDTR descriptor,
- handler-address splitting and reconstruction,
- an interrupt-gate option value of `0x8e00`,
- PIC offsets `0x20` and `0x28`,
- IRQ4 vector derivation as `0x24`,
- masks that expose only master IRQ4.

Host tests validate these byte-level contracts. Bare-metal code owns the static
256-entry table, reads the current code selector, installs the IRQ4 gate, and
loads IDTR with `lidt` while interrupts are disabled.

## Interrupt Controller Contract

The architecture adapter remaps the legacy 8259 pair using the standard ICW1
through ICW4 sequence. The slave PIC remains fully masked and the master exposes
only IRQ4. UART initialization already asserts the 16550 OUT2 gate; the adapter
then enables only the THRE interrupt source.

After the interrupt is observed, the adapter disables CPU interrupts and masks
the PIC again. V0 does not leave a general interrupt runtime active after the
one-shot proof.

## Top Half

The IRQ4 entry is a small `global_asm` stub. It saves only the registers it
uses, then:

1. reads COM1 IIR to identify and clear the THRE source,
2. reads COM1 LSR,
3. writes both bytes and a seen flag into fixed architecture-owned atomics,
4. disables COM1 IER,
5. sends a non-specific EOI to the master PIC,
6. restores registers and returns with `iretq`.

It does not call Rust, allocate, print, acquire kernel authority, or mutate
Agent Kernel records. This keeps interrupt-stack and reentrancy requirements
small and explicit.

## Bottom Half

The interrupted boot context waits with a bounded spin, then disables CPU
interrupts and validates the mailbox:

- exactly one IRQ was observed,
- IIR reports an interrupt pending with THRE identity,
- LSR reports transmitter empty.

Only then does normal Rust code raise `DeviceEventKind::Interrupt` under the
bootstrap capability. The payload stores raw IIR in `code` and LSR in `value`.
Delivery queues the Driver Invocation; dispatch, tick, acknowledgement, causal
write execution, command completion, and invocation completion follow the
existing kernel contract.

## QEMU Proof

The boot entry emits this marker only after the validated assembly handler has
run:

```text
AGENT_KERNEL_UART_IRQ_OK
```

The prior polling command disappears. The deterministic kernel trace becomes:

- events 1 through 15: bootstrap, endpoint, and Driver admission,
- event 16: interrupt Device Event raised,
- events 17 through 21: delivery, invocation queue/dispatch/tick, acknowledge,
- events 22 through 24: causal write submit/dispatch/complete,
- event 25: Driver Invocation completed.

## Failure Semantics

IDT encoding, selector installation, PIC setup, or UART arming failure occurs
with CPU interrupts disabled. A bounded wait prevents a missing IRQ from hanging
QEMU forever. An absent, duplicate, non-THRE, or non-empty-transmitter signal is
rejected before a Device Event is created. Backend failures still use the
terminal command failure syscall. No interrupt-context code edits kernel state.

## Unsafe Boundary Audit

The privileged boundary is limited to the architecture binary. The static IDT
is mutated only while IF is clear during single-core boot and remains live for
the image lifetime. Its gate points to a compiled assembly symbol using the
current code selector.

The assembly entry saves and restores the only general registers it modifies,
makes no calls, receives no error code for external IRQ4, disables the UART
source before the master-PIC EOI, and ends in `iretq`. Compiled disassembly was
checked for the expected `0x3fa`, `0x3fd`, `0x3f9`, and `0x20` port accesses.
The mailbox uses byte atomics; the handler and interrupted context execute on
the same core, and normal code reads payload fields only after observing the
seen flag and clearing IF again. No reference into kernel-owned state crosses
the interrupt boundary.

## Deferred Work

V0 installs only one hardware gate and masks it after one interrupt. It does not
yet provide exception handlers, a general IRQ registry, nested interrupts,
interrupt-safe queues, APIC/IOAPIC, SMP routing, timer preemption, userspace
return, UART receive buffering, or dynamically assigned interrupt capabilities.
Those become separate native-kernel milestones after this ingress path is
proven.
