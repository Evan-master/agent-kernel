# X86 Agent Fault Containment V0 Design

## Status

Implemented, validated, merged, and published on 2026-07-16.

## Purpose

The native runtime can now re-arm a physical PIT quantum before every ring-3
entry, but vector 6 still uses the global fatal exception handler. One invalid
instruction in an Agent therefore terminates the complete kernel image. This
milestone contains a ring-3 invalid-opcode exception as a kernel-visible
`ExecutionTrap`, removes only the faulting CPU context, and proves that another
queued Agent continues to execute.

The proof adds one immutable Fault Worker Capsule. Its first run waits on the
read-only physical quantum generation and is preempted normally. On its second
dispatch it executes `ud2`. The Verifier is queued behind the Fault Worker,
receives its own admission preemption, and completes only after the Fault
Worker has entered `TaskStatus::Faulted`.

## Layer Placement

- `agent-kernel-core` keeps sole ownership of `FaultRecord`, `TaskFaulted`,
  task status, execution-context status, and event ordering. Its behavior does
  not change.
- `agent-kernel` remains the only mutation facade; the x86 executor calls
  `sys_fault_task` and never edits task or fault storage directly.
- `agent-kernel-boot` exposes an optional trailing fixed fault capacity while
  preserving zero-capacity compatibility for every existing instantiation.
- The host-testable `agent-kernel-x86_64` library extends the strict native run
  boundary contract with one supported architecture fault.
- The bare-metal x86 adapter owns the vector-6 gate, register capture, CR3
  switch, frame validation, physical context disposal, and boot evidence.

## Physical Boundary Contract

One reset mailbox may prove exactly one of three mutually exclusive returns:

1. `AgentCall`: one call, no timer IRQ, and no Agent fault.
2. `QuantumExpired`: one timer IRQ with preemption evidence, no call, and no
   Agent fault.
3. `AgentFault(InvalidOpcode)`: one vector-6 Agent exception, no call, no timer
   IRQ, and no preemption marker.

Empty, mixed, repeated, inconsistent, or unsupported-vector evidence fails
closed. Classification occurs before call decoding, tick mutation, or fault
mutation.

## Exception Entry

The persistent IDT vector-6 entry is replaced during `AgentCpuRuntime`
installation while interrupts are clear. The assembly stub first preserves
`rax`, reads the saved CS privilege bits, and branches by origin:

- CPL3 saves the complete integer register frame, switches from Agent CR3 to
  kernel CR3, records vector/RIP/RSP/CR3 evidence, and restores the saved host
  context.
- CPL0 restores `rax` and jumps to the original fatal vector-6 handler. A
  kernel invalid opcode therefore remains an image-fatal invariant violation.

The handler does not call Rust while an Agent frame is active and does not
acknowledge the PIC because #UD is a CPU exception, not an IRQ.

## Validation And Ownership

`FaultedAgentCpu::capture` accepts the physical return only when all of the
following hold:

- the strict boundary classifier reports vector 6;
- host context, Agent CR3, captured RSP, and captured RIP are coherent;
- the copied frame is a valid CPL3 frame inside the owning Capsule and stack;
- the shared RSP0 canary, active kernel CR3, CPL, and interrupt state are valid;
- the fault RIP lies inside the owning verified Capsule code mapping.

The resulting object owns the memory and frame until the executor successfully
calls `sys_fault_task(agent, task, ExecutionTrap, 6)`. It is then consumed and
recorded as terminal fault evidence; it is never parked as resumable state.
If semantic mutation fails, execution stops rather than losing ownership.
The boot evidence adapter separately requires the terminal RIP to equal the
immutable proof Capsule's `ud2` offset 16; that role-specific constant does not
enter the generic CPU capture layer.

## Boot Proof

The Fault Worker is Agent 6 with its own delegated task, verified Worker image,
private P4 root, signal page, guard page, and stack. After both normal Workers
complete, the boot adapter queues Fault Worker then Verifier:

1. Fault Worker dispatches and its admission quantum expires.
2. Verifier dispatches and its admission quantum expires.
3. Fault Worker resumes, observes physical generation 1, and executes `ud2`.
4. The executor records one `TaskFaulted` event with `ExecutionTrap`, detail 6,
   and leaves its execution context `Faulted`.
5. Verifier redispatches after that event and completes its existing inspection,
   verification, intent fulfillment, and completion flow.

Terminal evidence requires eleven dispatches, four prepared normal/fault
contexts, five preempted contexts, five physical quantum expiries, one Agent
fault boundary, three completed contexts, one faulted context, an empty native
runtime, and an empty run queue. The exact trace contains 101 events.

## Failure Policy

Any unsupported vector, mixed mailbox state, malformed frame, wrong CR3, wrong
fault offset, failed `sys_fault_task`, missing fault record, incorrect status,
or inability to continue the Verifier path terminates through the existing
explicit boot failure path. No fault is inferred from a timeout or log line.

## Validation

- Host tests cover all three valid boundaries and reject mixed, repeated,
  inconsistent, and unsupported fault evidence.
- Boot crate tests prove the new trailing fault capacity reaches the public
  facade while default zero-capacity behavior remains unchanged.
- Full workspace tests, Supervisor output, formatting, no_std checks, and
  scoped Clippy remain green.
- Debug and release QEMU require
  `AGENT_KERNEL_NATIVE_AGENT_FAULT_CONTAINMENT_OK` and the exact 101-event
  sequence.
- Release disassembly must show CS-origin discrimination, full register
  capture, Agent-to-kernel CR3 switching, and host-context restoration.

## Non-Goals

V0 contains only a no-error-code ring-3 invalid-opcode exception. It does not
recover or restart the faulting task, route a fault policy, contain page faults
or general protection faults, decode CPU error codes, install IST stacks,
handle double faults, tear down page tables, or provide SMP exception routing.
