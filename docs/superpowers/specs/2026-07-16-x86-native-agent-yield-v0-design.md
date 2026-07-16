# X86 Native Agent Yield V0 Design

## Status

Accepted for autonomous implementation on 2026-07-16.

## Purpose

The native Agent Call ABI already decodes `Yield`, and the core scheduler can
move a running task to the back of its FIFO queue. The physical x86 path does
not connect those contracts. A ring-3 Agent that issues Yield therefore has no
owned CPU type-state, no semantic transition, and no way to resume at the call
return point.

This milestone implements one complete cooperative handoff. Worker A sends a
message that wakes Worker B, then yields. The kernel appends A behind B, the
native runtime parks A's call frame, and B resumes from its mailbox wait. After
B completes, kernel-selected dispatch recovers A's yielded frame. A receives a
successful Yield reply and then completes.

## Layer Placement

- `agent-kernel-core` retains the existing deterministic `yield_task`
  transition and FIFO policy. No architecture state enters the core.
- `agent-kernel` retains the existing `sys_yield_task` facade boundary.
- The architecture library adds the canonical Yield success reply and owned
  sender CPU type-states.
- The bare-metal x86 adapter owns parked Yielded frames, readiness checks, and
  the concrete Worker schedule proof.
- The Worker capsule remains native ring-3 code and makes no POSIX call.

## Agent Call Contract

Worker A issues `AGENT_CALL_YIELD` only after a successful SendMessage reply.
The request must echo the scheduler-owned Agent, Task, Image, and nonce and must
leave all reserved registers canonical. `AgentCallContext::matches_yield`
continues to be the authority check.

The kernel encodes a Yield success reply only after `sys_yield_task` succeeds.
The reply uses the existing ABI envelope:

- `RAX = AGNTCALL`;
- `RBX = ABI version 1`;
- `RCX = status OK`;
- `RDX = AGENT_CALL_YIELD`;
- `RSI/RDI/R8/R9 = trusted Agent/Task/Image/nonce`;
- `R10-R15/RBP = 0`.

On resume, Worker A checks the returned operation before issuing CompleteTask.

## Physical Ownership

Capturing a valid Yield request creates `RequestedSenderYieldCpu`. It still
owns Worker A's complete saved privilege frame and prior DescribeContext,
SubmitTaskResult, and SendMessage evidence.

After the core transition succeeds, acknowledging the request writes the
success reply into that owned frame and produces `YieldedMailboxSenderCpu`.
That type is non-Copy and can only be moved into or out of the native runtime
registry. Resuming it consumes the token, returns through the saved frame, and
captures the following CompleteTask request as `CompletedMailboxSenderCpu`.

The sender sequence grows from four calls to five and from eight address-space
switches to ten. Its return offsets become DescribeContext, SubmitTaskResult,
SendMessage, Yield, and CompleteTask.

## Runtime Registry

`NativeAgentContext` gains a `Yielded` variant and
`NativeAgentContextKind::Yielded`. The context uses the same trusted
`AgentCallContext` identity as Prepared, Preempted, and WaitingMailbox states.

The runtime adds one park and one guarded take operation for yielded senders.
The existing dispatch-readiness protocol must match Agent, Task, and Yielded
state before the semantic redispatch commit. The guarded take repeats those
checks after commit.

The fixed capacity remains three. At the high-water mark it contains the
Verifier's Prepared context, B's WaitingMailbox context, and A's Yielded
context. No heap allocation is introduced.

## Semantic Sequence

Immediately after MessageSent and MessageWaitWoken:

1. Validate A's Yield request and the pre-yield scheduler state.
2. Call `sys_yield_task(A)`, producing `TaskYielded`.
3. Verify queue order `[B, A]`, both task states, contexts, results, and event.
4. Encode A's successful Yield reply and park its Yielded frame.
5. Prepare B's dispatch permit and match its WaitingMailbox frame.
6. Commit B's dispatch and guarded-take that frame.
7. Let B receive, acknowledge, submit its result, and complete.
8. Verify queue order `[A]` and B's terminal state.
9. Prepare A's dispatch permit and match its Yielded frame.
10. Commit A's dispatch and guarded-take that frame.
11. Resume A through the Yield reply and capture CompleteTask.
12. Complete A and prove both Workers are terminal with an empty queue.

The prior direct A-complete-to-B-dispatch path is removed. No parallel
semantic and physical ownership path remains.

## Event Contract

The new ordered events are:

- event 59: `TaskYielded` for Worker A;
- event 60: `TaskDispatched` for Worker B;
- events 61 through 64: B mailbox/result/completion lifecycle;
- event 65: `TaskDispatched` for yielded Worker A;
- event 66: A completion.

Verifier events shift to 67 through 74 and Driver events shift to 75 through
84. The trace remains deterministic and must contain exactly 84 events.

## Capsule Contract

Worker A's code adds one canonical Yield request and verifies `RDX` in the
success reply. The capsule code length, total length, SHA-256 digest, expected
return offsets, call count, and address-space-switch count all change together.
Worker B and the Verifier capsules remain byte-identical.

## Failure And Atomicity

- Decode, identity, nonce, or scheduler-state failure happens before mutation.
- Core Yield failure leaves the frame unacknowledged and outside the registry.
- Registry insertion failure is a boot invariant violation after semantic
  Yield; the fixed-capacity proof prevents it in the demonstrated schedule.
- Missing B readiness prevents B's dispatch commit while both parked frames
  retain ownership.
- Missing A Yielded readiness prevents A's redispatch commit after B completes.
- Guarded take failure after commit remains a deterministic fail-stop invariant
  violation, matching the existing single-core protocol.

## Evidence

QEMU adds:

`AGENT_KERNEL_NATIVE_AGENT_YIELD_OK`

It is emitted only after event 59 records the FIFO yield, B completes between
the two A execution intervals, A resumes from the exact yielded frame, event 66
completes A, and the runtime contains only the prepared Verifier context.

## Validation

- A red ABI test requires canonical Yield reply encoding.
- Existing core scheduler tests continue to prove FIFO yield semantics.
- Host tests validate request/reply register canonicality.
- Debug and release QEMU require the new marker and exactly 84 ordered events.
- Full workspace tests, Supervisor, host and bare-metal Clippy pass.
- Release disassembly preserves CR3 and complete register-frame boundaries.

## Non-Goals

This V0 does not implement an unbounded runtime loop, arbitrary Agent programs,
multiple consecutive yields, yield cancellation, priorities, dynamic runtime
capacity, asynchronous completion, SMP synchronization, context migration,
terminal address-space reclamation, or hardware-fault recovery.
