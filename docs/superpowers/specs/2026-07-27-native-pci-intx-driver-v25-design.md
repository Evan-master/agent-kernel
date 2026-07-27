# Native PCI INTx Driver V25 Design

**Status:** implemented and verified

## Objective

V25 connects a physical PCI INTx signal to the native ring-3 Driver Agent
runtime and adds bounded Driver Invocation fault recovery.

```text
PCI serial INTA
  -> I/O APIC IRQ 11
  -> ring-0 one-shot top half
  -> immutable Interrupt Device Event
  -> ring-3 Driver Invocation
  -> semantic Driver Command
  -> ring-0 PCI serial backend
```

The interrupt handler owns no Core mutation. It captures fixed-width hardware
evidence, disables the device interrupt source, and leaves event creation,
delivery, scheduling, and recovery to the BSP execution flow.

## Frozen QEMU Hardware Profile

The V25 boot proof targets QEMU's single-port `pci-serial` device:

```text
BDF             0000:00:04.0
PCI ID          1b36:0002
BAR 0           8-byte I/O 16550 register window
interrupt pin   INTA
interrupt line  IRQ 11
trigger         level
polarity        active low
```

PCI inventory reads configuration DWORD `0x3c` and records both the interrupt
line and pin. A line value of `0xff`, pin zero, a pin outside `1..4`, or a line
outside the I/O APIC input range fails route construction.

V25 accepts the exact QEMU profile above. ACPI `_PRT`, PCI bridge swizzling,
MSI, and MSI-X remain later milestones.

## I/O APIC And IDT Ownership

The PCI route uses a dedicated IDT vector installed before the IDT is frozen.
Its redirection entry remains masked until all of these conditions hold:

1. PCI inventory and BAR ownership are verified;
2. the interrupt capture state contains the trusted BAR base;
3. the Driver Agent has completed its recoverable configuration Invocation;
4. the device source is armed through a completed `Configure` command.

Arming clears stale UART interrupt identification, enables only the 16550 THRE
interrupt, then unmasks IRQ 11. The I/O APIC route is active-low and
level-triggered.

The top half:

1. reads IIR and LSR from the trusted BAR base;
2. disables the UART interrupt-enable register;
3. publishes one pending capture with release ordering.

After control returns with IF clear, the BSP masks IRQ 11 and sends Local APIC
EOI before consuming the capture. Delaying EOI keeps the level-triggered route
in service while the source is being disabled.

Repeated entry while a capture is pending is a fatal invariant violation.
Normal code consumes the capture once and raises an immutable
`DeviceEventKind::Interrupt`.

## Driver Command ABI

Agent Call operations `1..60` keep their assigned values. Operation `59`
accepts these Driver command wire kinds:

```text
1  Write
2  Configure
```

`Configure` uses a semantic opcode owned by `PciSerialBackend`:

```text
opcode  ARM_THRE_INTERRUPT
value   0
```

The backend translates the command into bounded 16550 register operations.
Ring 3 never receives the BAR base and has no raw port-I/O primitive.

`Write` retains the V24 contract:

```text
opcode  WRITE_THR
value   0x50
```

## Two-Invocation Boot Proof

V25 admits the same immutable Driver Capsule for two independent native
Invocations.

### State-Change Invocation

The first Device Event has kind `StateChanged` and carries the PCI identity.
The Capsule:

1. describes and inspects its authenticated Driver context;
2. reads the restart generation from its private signal page;
3. executes `ud2` when generation is zero;
4. after recovery, acknowledges the event;
5. submits `Configure(ARM_THRE_INTERRUPT, 0)`;
6. completes the Invocation.

The kernel retains exclusive ownership of the same address-space frames across
the restart. It clears the signal, stack, lazy-data, and call-data pages before
preparing generation one. Code and read-only image mappings remain immutable.
The complete Invocation then follows the normal quarantine, TLB shootdown, and
frame-reclamation path.

### Interrupt Invocation

After the configuration Invocation completes, ring 0 unmasks IRQ 11. The real
INTx capture becomes a second Device Event. The same Capsule branches on event
kind, then:

1. acknowledges the interrupt event;
2. submits `Write(WRITE_THR, 0x50)`;
3. validates the backend result;
4. completes the Invocation.

The physical serial backend must emit exactly one byte, `0x50`.

## Driver Fault State

`DriverInvocationStatus` gains `Faulted`. Each record stores:

- the most recent `FaultKind` and detail;
- a bounded restart generation;
- its existing deterministic tick and quantum state.

`fault_driver_invocation` is valid only for the currently running Driver and
Invocation. It clears the quantum, changes the execution context to a
Driver-scoped `Faulted` state, and emits `DriverInvocationFaulted`.

`recover_driver_invocation` requires an active actor with `Rollback` authority
over the Invocation resource. Recovery is allowed only when:

- status is `Faulted`;
- the Device Event remains `Delivered`;
- no Driver Command is bound to the Invocation;
- restart generation is zero.

Recovery increments the generation, returns the Invocation to `Queued`, clears
the Driver execution context, and emits `DriverInvocationRecovered`.

A second fault, an acknowledged event, any command evidence, a wrong actor,
or insufficient authority fails without mutation. V25 performs at most one
restart per Invocation.

## Native CPU Recovery

The x86 Driver executor exposes a `Faulted` run result containing the native
trap evidence and parked CPU owner. It performs these steps:

1. commits the Core fault transition;
2. verifies the fault occurred after `DescribeContext` and
   `InspectDriverInvocation`;
3. clears the retained private frames and increments the restart generation;
4. commits owner-authorized Core recovery;
5. prepares the identical verified Capsule in the retained address space;
6. resumes through normal FIFO dispatch.

No Agent Call may acknowledge the Device Event or submit a command before the
intentional first-generation trap.

## Event Contract

V25 adds stable archive tags:

```text
90  DriverInvocationFaulted
91  DriverInvocationRecovered
```

The frozen V25 boot suffix is `418..451`. The state-change Invocation occupies
`425..440`; the interrupt Invocation occupies `441..451`. Debug and release
QEMU must reproduce all `451` Events exactly.

## Failure Semantics

- Invalid PCI interrupt metadata prevents route creation.
- A mismatched BDF, PCI ID, BAR, interrupt pin, or line stops the V25 proof.
- IRQ 11 remains masked until device configuration completes.
- An unsupported command kind or semantic opcode produces no hardware write.
- A pending-capture collision stops boot.
- Core event, scheduling, command, and fault transitions are fail-before-write.
- Fault recovery is rejected after any irreversible Driver side effect.
- A second Driver fault is terminal for the V25 boot proof.
- Missing interrupt delivery, wrong IIR evidence, or a wrong serial byte fails
  the QEMU gate.

## Verification

V25 verification covers:

- PCI line and pin parsing;
- active-low, level-triggered I/O APIC route encoding;
- Driver Agent Call `Configure` decoding and reserved-register checks;
- PCI serial backend interrupt arming;
- Driver fault, recovery, rejection, and event archive contracts;
- exact Driver Capsule assembly and digest;
- native trap context and restart-generation evidence;
- two address-space reclamation proofs;
- real IRQ 11 capture and one-byte serial output;
- full workspace tests, strict host and bare-metal Clippy, supervisor,
  shell checks, image audit, debug QEMU, and release QEMU.
