# X86 Exception Baseline V0 Design

## Purpose

The first UART interrupt milestone installs a 256-entry IDT but populates only
IRQ4. That proves hardware ingress, yet any CPU exception after `lidt` still
targets a missing gate and can collapse into a triple fault.

This milestone makes the IDT a persistent architecture-owned runtime object,
installs explicit entries for exception vectors 0 through 31, and proves a
returning breakpoint frame in QEMU before the Agent Kernel bootstrap proceeds.
The same table then receives the UART IRQ4 gate.

```text
kernel `int3`
  -> vector 3 trap gate
  -> assembly breakpoint top half
  -> capture CPU return RIP in a fixed mailbox
  -> `iretq`
  -> validate exact expected RIP
  -> continue Agent Kernel boot
```

## Architecture Placement

Portable IDT byte encoding remains in the no_std `agent-kernel-x86_64` library
so host tests can verify it without privileged instructions. A new
architecture-binary exception runtime owns the static table, `lidt`, exception
assembly, and probe mailbox. Kernel core records, capabilities, and scheduling
remain untouched.

The probe runs before `AgentKernel` bootstrap. Its fixed mailbox is
architecture-local self-test state rather than Agent semantic state, so it does
not append a kernel event. This is an explicit exception to the normal
kernel-mutation event rule.

## Gate Contract

The existing long-mode 16-byte gate layout and 10-byte IDTR descriptor remain
unchanged. V0 adds a present ring-0 trap-gate option value of `0x8f00` for the
breakpoint vector. All fatal exceptions use the existing ring-0 interrupt-gate
value `0x8e00`, which clears IF on entry.

The static table has 256 entries and remains alive for the kernel image
lifetime. Gate publication uses volatile stores because the CPU consumes the
table outside Rust's normal memory model. Installation occurs once with IF
clear. The UART adapter later writes vector `0x24` in that same table while IF
remains clear; it no longer allocates, owns, or reloads a private IDT.

## Breakpoint Probe

The breakpoint stub is deliberately minimal:

1. save `RAX`, the only general register it uses,
2. copy the CPU-pushed return RIP from the interrupt frame,
3. increment a byte count and set a seen flag,
4. restore `RAX`,
5. return through `iretq`.

The caller computes the exact address of the instruction following `int3` in
the same inline assembly block. After return, normal Rust validates one entry,
the seen flag, and equality between captured and expected RIP. The success
marker is emitted only after all checks pass:

```text
AGENT_KERNEL_EXCEPTION_BASELINE_OK
```

## Fatal Exception Entries

Every exception vector other than 3 has a dedicated non-returning assembly
stub. Each stub stores its vector number, increments a fault count, writes the
QEMU failure value to `isa-debug-exit`, then executes a `cli`/`hlt` loop if no
emulator exit device is present.

Because fatal stubs never unwind or return, they do not need to normalize the
different CPU frames used by exceptions with and without hardware error codes.
This avoids a false recovery contract while still replacing missing gates with
deterministic failure paths.

## UART Integration

The UART IRQ4 assembly top half and PIC/16550 sequence stay unchanged. Its
adapter requests vector `0x24` installation from the exception runtime before
unmasking IRQ4. The runtime rejects that request until the baseline IDT has been
loaded. After the one-shot UART proof, exception gates remain installed even
though IF and the PIC are masked again.

The kernel event trace remains exactly 25 events because the exception probe is
architecture initialization, not an Agent operation.

## Failure Semantics

IDT construction or loading failure leaves IF clear and aborts boot with the
normal QEMU error path. A missing, duplicate, or wrong-RIP breakpoint signal is
rejected before kernel bootstrap. Any other installed CPU exception exits QEMU
with failure rather than returning into unknown state.

## Deferred Work

V0 does not recover fatal exceptions, decode error codes, read `CR2`, preserve a
full register frame, route hardware exceptions into Agent Fault records, install
a TSS/IST stack for double fault, support privilege transitions, handle NMIs or
machine checks specially, or provide SMP-safe IDT updates. Those contracts must
precede user mode and untrusted Agent execution. PIT timer IRQ0 and scheduler
preemption become the next hardware milestone after this baseline is proven.
