# X86 Resumable Runtime Registry V1 Design

## Status

Implemented, validated, merged, and published on 2026-07-16.

## Purpose

Kernel-selected dispatch V0 makes the first physical dispatch of Worker A,
Worker B, and the Verifier follow the Agent/Task identity returned by the core.
After first dispatch, however, preempted and mailbox-waiting CPU objects still
move through named local variables. Later semantic dispatches validate their
identity but do not use the dispatch result to recover physical ownership.

This milestone makes every demonstrated x86 redispatch select a parked native
context from one bounded runtime registry. A context is parked whenever it is
not executing on the CPU and is still needed by a non-terminal task. The core
continues to own runnable order and task lifecycle; the x86 adapter owns only
the physical context matching that semantic identity.

## Layer Placement

- `agent-kernel-core` and the `agent-kernel` facade remain unchanged. Their
  kernel-selected FIFO dispatch contract is already sufficient.
- `agent-kernel-x86_64` library code keeps the allocation-free generic
  ownership store and adds only host-testable atomic selection support when
  needed.
- The bare-metal x86 adapter owns a private enum over concrete CPU type states
  and converts between enum variants and the existing typed execution flows.
- Boot flow modules park and take contexts around public semantic syscalls. They
  do not acquire scheduler policy or mutate core internals.

## Runtime Context Model

The bare-metal registry owns at most three parked contexts, keyed by the Agent
ID in each trusted `AgentCallContext`:

1. `Prepared(PreparedAgentCpu)` before an Agent's first physical entry.
2. `Preempted(PreemptedAgentCpu)` after a PIT interrupt and before redispatch.
3. `WaitingMailbox(WaitingMessageReceiveCpu)` while the receiver task is in the
   kernel's mailbox Waiting state.

Exactly one registry slot may exist for an Agent. A physically running context
is absent from the registry because its ownership is held by the active boot or
call-flow token. A terminal context is not reinserted.

Each variant exposes only its trusted Agent/Task identity and variant kind to
the adapter. The generic store remains unaware of scheduler state and x86 CPU
types.

## Atomic Selection Contract

Every take operation receives the exact `RunQueueEntry` returned by
`sys_dispatch_next_ready_with_quantum` and an expected physical state. It must:

1. find the entry by the returned Agent ID;
2. verify the parked trusted context has the same Agent and Task IDs;
3. verify the parked variant is the state expected by that transition;
4. remove and return exactly one owned CPU object only after all checks pass.

Missing Agent, mismatched Task, or mismatched physical state leaves registry
length, order, and ownership unchanged. Failed insertion returns the rejected
non-Copy context to the caller.

The boot adapter may know which physical state is legal at a type-state
transition, but it may not choose the Agent. Agent selection always starts with
the core's returned queue entry.

## Demonstrated Ownership Schedule

The existing semantic event sequence remains exactly 82 events. Physical
ownership changes as follows:

1. Register prepared A, B, and Verifier contexts.
2. Dispatch B from the core and take prepared B.
3. Park preempted B, dispatch A, and take prepared A.
4. Park preempted A, dispatch B, and take preempted B.
5. Convert B's receive call into a mailbox waiter and park waiting B.
6. Dispatch A and take preempted A.
7. A sends the message, waking B while B remains parked.
8. Complete A, dispatch B, and take waiting B.
9. Complete B; only the prepared Verifier remains parked.
10. Dispatch and take prepared Verifier, then park it after PIT preemption.
11. Redispatch Verifier and take its preempted context.
12. Complete Verifier with an empty registry.

No `PreemptedAgentCpu` or `WaitingMessageReceiveCpu` may cross a semantic
dispatch as a separate named local selected by the caller.

## Failure Model

Boot remains fail-stop. Registration, parking, identity, state, capacity, or
take failures emit the existing native-runtime fatal marker and terminate. The
generic store exposes explicit deterministic errors and never panics for normal
lookup, capacity, duplicate, or state-selection failures.

Parking happens before the semantic transition that makes another Agent
Running. Therefore a later failure cannot silently lose the suspended physical
context. The boot proof terminates instead of attempting rollback of hardware
execution evidence.

## Evidence

QEMU adds one marker:

`AGENT_KERNEL_RESUMABLE_RUNTIME_REGISTRY_OK`

It is emitted only after the Worker preemption, mailbox wait/wake, and Verifier
preemption paths have all recovered the correct context from kernel-selected
dispatch results and the registry is empty at the terminal boundary.

The existing kernel-selected and prepared-store markers remain required. The
event trace must still contain exactly 82 ordered events.

## Validation

- Red tests for guarded store selection prove mismatch atomicity and ownership.
- Existing generic store ownership, duplicate, capacity, and compaction tests
  remain green.
- Host workspace tests and Supervisor output remain unchanged.
- `x86_64-unknown-none` no_std checks and scoped Clippy pass.
- Debug and release QEMU runs require the new marker and exactly 82 events.
- Release disassembly still proves Agent CR3 entry/resume and interrupt return
  boundaries.

## Non-Goals

This V1 does not implement a general scheduler executor, dynamic Agent count,
heap allocation, SMP, context migration, terminal context reclamation,
page-table teardown, asynchronous kernel-call continuation, or persistence
across reboot. CPU call-flow tokens may remain locally owned during one
synchronous Agent-call transaction because no other Agent is dispatched within
that transaction.
