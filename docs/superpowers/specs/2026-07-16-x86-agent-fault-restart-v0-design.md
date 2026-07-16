# X86 Agent Fault Restart V0 Design

## Status

Implemented, validated, merged, and published on 2026-07-16.

## Purpose

The x86 runtime can contain a ring-3 invalid opcode and keep an unrelated
Verifier running, but the faulting task remains terminally `Faulted`. The core
already exposes rollback-authorized `recover_faulted_task`; this milestone
connects that semantic transition to a fresh physical Agent execution context.

The proof Fault Worker faults once, is recovered by the bootstrap Agent through
its explicitly granted rollback capability, is requeued by its assigned Agent,
starts again at the immutable Capsule entry, and completes normally. The
original exception frame is never resumed.

## Layer Placement

- `agent-kernel-core` remains the sole owner of rollback authorization,
  `TaskFaultRecovered`, task status, execution-context state, and event order.
  Its behavior does not change.
- `agent-kernel` remains the public mutation facade. The x86 adapter uses
  `sys_recover_faulted_task` and `sys_enqueue_task`; it does not edit semantic
  stores directly.
- `agent-kernel-boot` adds `Rollback` to the explicitly enumerated bootstrap
  capability so the system owner can authorize recovery without a bypass or an
  architecture-private capability.
- The host-testable x86 library defines one additional byte in the read-only
  Agent signal ABI for a kernel-authored restart generation.
- The bare-metal x86 adapter owns mutable-memory reset, consumption of the
  faulted CPU object, construction of a new prepared CPU object, physical
  runtime registration, and boot evidence.

## Recovery Authority

Only an active Agent holding `Operation::Rollback` authority over the task
resource may recover the task. The proof uses the bootstrap Agent and its
explicitly granted bootstrap capability. The assigned Fault Worker may enqueue
only after recovery has moved the task from `Faulted` to `Accepted` and cleared
its semantic execution context to `Idle`.

Physical restart preparation does not imply semantic recovery. The adapter
first validates and prepares a replacement physical context, then invokes the
public recovery syscall, registers the replacement in the bounded runtime, and
queues the recovered task. Any failure stops the boot proof rather than
creating an accepted task without executable ownership.

## Restart Signal ABI

The private signal page remains read-only to ring 3 and writable only through
the kernel's supervisor physical alias:

- byte 0: Agent-call release;
- byte 1: physical quantum generation;
- byte 2: restart generation.

Initial memory preparation clears all three bytes. A fault restart clears the
complete signal page and every writable stack page, then writes restart
generation 1. The code page and page-table mappings remain unchanged and are
revalidated through the existing address-space roots. V0 supports exactly one
restart, so zero means initial execution and one means restarted execution.

## Physical Type Transition

`FaultedAgentCpu` owns the validated exception frame, Agent memory, runtime
boundary, call identity, and fault metadata. Its restart operation consumes the
whole value. It does not expose the saved frame and cannot produce a resumable
frame.

After mutable-memory reset, the operation passes the owned memory back through
`AgentCpuRuntime::prepare_restarted`, producing `PreparedAgentCpu`. The next
entry therefore uses the Capsule entry RIP, stack top, sanitized registers, and
a new per-dispatch mailbox. The first restarted dispatch must expire a real PIT
quantum before any Agent Call, preserving the admission boundary used by every
other prepared Agent.

## Fault Worker Capsule

The immutable proof Capsule has two paths after observing physical quantum
generation 1:

1. restart generation 0 executes `ud2` at the fixed proof offset;
2. restart generation 1 performs `DescribeContext`, then authenticated
   `CompleteTask`.

The first run is admitted, preempted, resumed, and faulted. Recovery clears its
mutable pages and sets restart generation 1. The replacement context is
admitted, preempted, resumed from the fresh entry flow, authenticates its own
Agent/Task/Image identity, and completes through the ordinary call executor.

## Boot Proof

The existing first phase still completes Worker A and Worker B. The second
phase still queues Fault Worker before Verifier and proves the Verifier
continues after `TaskFaulted`. Once that queue drains:

1. validate the terminal fault object, fault record, and Verifier completion;
2. consume the fault object into a prepared replacement context;
3. call `sys_recover_faulted_task` with bootstrap rollback authority;
4. register the prepared context and enqueue the recovered Fault Worker;
5. dispatch it to one physical quantum expiry;
6. redispatch it to `DescribeContext` and `CompleteTask`;
7. require `TaskFaulted < TaskFaultRecovered < TaskQueued < TaskCompleted`.

Terminal evidence requires thirteen kernel-selected dispatches, five prepared
initial/restart contexts, six preempted contexts, six physical quantum
expiries, one Agent fault, four completed contexts, no faulted physical
contexts, an empty native runtime, and an empty run queue. The exact event
count is expected to grow from 101 to 107.

## Failure Policy

Recovery fails closed on missing rollback authority, stale semantic state,
wrong fault identity, nonzero initial restart generation, unsupported restart
generation, dirty signal or stack reset, wrong address-space roots, inability
to construct or register a replacement context, wrong queue order, a second
fault, or a completion that does not authenticate the original call context.

The semantic fault record remains immutable after recovery. V0 retains the
same physical pages only after clearing all Agent-writable state; it does not
allocate a replacement address space or reclaim frames.

## Validation

- Host tests lock the three signal offsets and their disjoint in-page layout.
- Bare-metal behavior proves the consumed fault-to-prepared type transition,
  reset generation, fresh admission preemption, and authenticated completion.
- Full workspace tests, Supervisor output, formatting, no_std checks, and
  scoped Clippy remain green.
- Debug and release QEMU require a new restart marker and the exact 107-event
  sequence.
- Release disassembly must continue to show the CPL-aware #UD path and fresh
  user-entry path; the existing fault frame must not be resumed for restart.

## Non-Goals

V0 does not implement automatic retry policy, multiple restart generations,
checkpoint data restoration, replacement physical-page allocation, page-table
reclamation, fault-handler routing, restart backoff, crash loops, exceptions
other than ring-3 #UD, SMP migration, or persistent recovery across reboot.
