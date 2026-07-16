# X86 Dispatch Readiness Handoff V0 Design

## Status

Accepted for autonomous implementation on 2026-07-16.

## Purpose

Kernel-selected dispatch now returns the correct Agent/Task and every parked
x86 context is recovered through the native runtime registry. The semantic
dispatch currently commits first, though, and the boot adapter checks or takes
the physical context afterward. If that context is absent or in the wrong
state, the task is already `Running` and the boot proof can only fail-stop.

This milestone adds a two-phase single-core handoff. The core prepares an
opaque dispatch permit without mutation. The x86 adapter uses the permit's
exact queue-head identity to prove that the required parked CPU context exists.
Only then does the core commit the scheduler transition. The matching physical
context is consumed immediately after that successful commit.

## Layer Placement

- `agent-kernel-core` owns the permit model, read-only dispatch validation, and
  commit revalidation because task lifecycle and FIFO policy belong there.
- `agent-kernel` exposes syscall-style prepare and commit methods without
  inspecting or mutating core internals.
- `agent-kernel-x86_64` owns physical-context readiness checks for Prepared,
  Preempted, and WaitingMailbox states.
- Boot semantic flows compose both boundaries but do not acquire scheduling
  policy or bypass public facade operations.

## Core Permit Contract

The core adds an opaque copyable `TaskDispatchPermit` containing one
`RunQueueEntry` and nonzero quantum. Public getters expose both values; only the
core can construct a permit.

`prepare_next_ready_dispatch_with_quantum(quantum)` is read-only. It validates:

1. nonzero quantum;
2. a nonempty run queue;
3. an active queue-head Agent;
4. an idle execution context for that Agent;
5. an Accepted task assigned to that Agent;
6. live runtime admission for that Agent/Task;
7. capacity for the dispatch event.

Success returns a permit and changes no task, execution context, queue entry,
counter, or event. Failure returns the same deterministic `KernelError` the
single-step dispatch would return.

`commit_ready_dispatch(permit)` verifies that the current queue head still
equals the permit entry, repeats all dispatch eligibility checks, and then uses
the existing atomic mutation path. A stale permit returns `TaskNotRunnable`
without mutation. Success returns the exact committed `RunQueueEntry`.

The existing `dispatch_next_ready_with_quantum` compatibility API becomes
prepare followed immediately by commit, preserving behavior and error order.

## Native Readiness Contract

The x86 native runtime adds read-only readiness checks over its single parked
context store. A check receives the permit's exact Agent/Task entry and one
expected physical state:

- Prepared for first Worker or Verifier entry;
- Preempted for timer redispatch and mailbox sender recovery;
- WaitingMailbox for mailbox receiver wake recovery.

Readiness succeeds only when the trusted `AgentCallContext` has the same Agent
and Task and the parked enum variant matches. It never removes, reorders, or
changes a context. The existing guarded take repeats these checks after commit.

This is a single-core protocol: no runtime-registry operation occurs between
readiness and guarded take. Interrupt handlers do not mutate the registry. A
future SMP design requires versioned reservations or a lock protocol and is not
implied by this V0.

## Handoff Sequence

Every demonstrated x86 task dispatch follows:

1. Core prepares a permit for its current FIFO head.
2. The semantic flow confirms the permit entry is the expected task transition.
3. Native runtime confirms a matching parked physical state exists.
4. Core commits the permit and returns the same Agent/Task.
5. Native runtime guarded-take transfers that exact physical context.

The seven covered paths are initial Worker B, first Worker A, preempted Worker B,
preempted Worker A after mailbox wait, waiting Worker B after wake, initial
Verifier, and preempted Verifier.

Preemption paths park the outgoing context before preparing the next permit.
Mailbox wait parks the receiver before preparing the sender permit. Terminal
completion needs no outgoing park.

## Failure And Atomicity

- Core prepare failure leaves all semantic state unchanged.
- Native readiness failure prevents commit, so the queue head remains Accepted.
- A stale permit cannot dispatch a different FIFO entry.
- Core commit failure leaves the parked physical context untouched.
- Guarded take repeats identity and state checks; an unexpected failure is a
  boot invariant violation and remains fail-stop.
- No new kernel event represents permit preparation because it is a read-only
  validation, not a lifecycle mutation.

## Evidence

QEMU adds:

`AGENT_KERNEL_DISPATCH_READINESS_HANDOFF_OK`

It is emitted only after every Worker and Verifier physical handoff has used a
permit preflight and the terminal native registry is empty. The ordered event
trace remains exactly 82 events because prepare is intentionally invisible.

## Validation

- Red core tests prove prepare is read-only, commit dispatches the permit entry,
  and stale commit is atomic.
- Facade tests prove permit getters and syscall delegation.
- Native store tests prove read-only predicate matching does not transfer
  ownership.
- Existing scheduler compatibility tests remain unchanged and green.
- Full workspace tests, Supervisor, no_std checks, and scoped Clippy pass.
- Debug and release QEMU require the new marker and exactly 82 events.
- Release disassembly continues to prove CR3 and register-frame boundaries.

## Non-Goals

This V0 does not provide SMP synchronization, a general runtime loop, dynamic
Agent capacity, scheduler priorities, preemptive kernel threads, asynchronous
Agent-call continuations, recovery after a hardware context disappears, or
rollback of already executed ring-3 instructions.
