# X86 Native Agent Runtime Loop V0 Design

## Status

Accepted for autonomous implementation on 2026-07-16.

## Purpose

The physical x86 path can already enter isolated ring-3 Agents, capture their
calls, park non-Copy frames, and recover contexts selected by the kernel FIFO.
The remaining execution path is nevertheless encoded as separate Sender,
Receiver, and Verifier call sequences. Adding an operation or changing a
Capsule currently requires another role-specific chain of CPU structs and boot
methods.

This milestone replaces those fixed call chains with one bounded native Agent
runtime loop. The loop dispatches the Agent selected by the kernel, recovers
the matching physical context, captures the next decoded call, authorizes and
executes the operation through public kernel facades, writes a reply only after
successful semantic validation, and either resumes, waits, yields, or
terminates that same owned context.

The existing two Workers and Verifier remain the executable proof. Their
Capsules and semantic event trace stay byte-for-byte and event-for-event
compatible with Native Agent Yield V0.

## Layer Placement

- `agent-kernel-core` remains the sole owner of task, queue, mailbox,
  verification, intent, capability, and event semantics.
- `agent-kernel` remains the syscall-style facade. The runtime loop uses only
  its public methods.
- The reusable `agent-kernel-x86_64` library adds a fixed-capacity call
  transcript that can be host tested without privileged instructions.
- The bare-metal x86 adapter owns physical frames, dispatch-and-take, trusted
  supplemental authority, operation routing, and deterministic boot evidence.
- No POSIX process, thread, file, signal, or system-call model is introduced.

## Generic Call Transcript

`AgentCallTranscript<CAPACITY>` records each decoded call as an operation and
return offset. It is fixed-capacity, allocation-free, and independent of
Capsule roles. It exposes the exact call count, operation sequence, return
offsets, and derived address-space switch count.

Appending beyond capacity or using an invalid return offset fails without
changing the transcript. The transcript does not authorize calls and does not
contain capabilities. Final boot evidence compares transcripts with immutable
Capsule contracts, but the operation loop does not use those expected
sequences to decide what to execute.

## Generic CPU Ownership

The role-specific call-flow structs are replaced by four generic non-Copy CPU
states:

1. `PendingAgentCallCpu` owns a captured frame and decoded request whose
   semantic operation has not yet been acknowledged.
2. `ResumableAgentCpu` owns a frame containing a successful reply and can run
   until the next Agent call.
3. `WaitingAgentCallCpu` owns an unacknowledged ReceiveMessage frame plus the
   kernel waiter identity while its task is Waiting.
4. `CompletedAgentCpu` owns immutable context, nonce, and transcript evidence
   after a valid CompleteTask request consumes the live session.

The first call after the admission preemption must be DescribeContext. Its
nonce establishes the session. Every later request must match the trusted
Agent, Task, Image, and nonce before a semantic syscall can run.

Reply methods remain operation-specific. A Pending token can become Resumable
only through the reply encoder corresponding to its current request. A waiting
Receive frame remains unmodified until a message is actually received. Yield
is acknowledged only after the core has appended the task to the FIFO.

## Dispatch-And-Take Contract

`NativeAgentRuntime` gains one mutable dispatch operation that performs the
complete physical handoff:

1. prepare the read-only core dispatch permit;
2. read its kernel-selected Agent and Task;
3. prove the registry contains that exact trusted context;
4. commit the semantic dispatch;
5. guarded-take the same context before returning.

The caller no longer supplies an expected physical variant and cannot choose
the Agent. The returned `DispatchedNativeAgent` exposes the actual Prepared,
Preempted, WaitingMailbox, or Yielded state only after selection. A mismatch
before commit leaves both scheduler and registry unchanged. The single mutable
runtime borrow prevents registry mutation between readiness and take.

## Runtime Loop

The outer loop continues while the core run queue is non-empty:

- `Prepared` enters ring 3 until the admission PIT expiry, records the public
  tick transition, and parks `Preempted`.
- `Preempted` resumes the saved frame and captures the first DescribeContext
  request.
- `WaitingMailbox` retries the retained ReceiveMessage after kernel-selected
  wake dispatch.
- `Yielded` resumes through the already encoded Yield reply and captures the
  next call.

The inner call loop handles all ABI V1 operations:

- DescribeContext: establish the nonce, reply, and continue;
- SubmitTaskResult: submit through task-scoped authority, reply, and continue;
- SendMessage: send and wake through the core mailbox, reply, and continue;
- ReceiveMessage: receive and continue, or park the untouched call as Waiting;
- AcknowledgeMessage: acknowledge the owned message, reply, and continue;
- Yield: mutate the FIFO, encode success, park Yielded, and return to dispatch;
- InspectTaskResult: use trusted Verify authority, return the stored result,
  and continue;
- VerifyTask: use trusted Verify authority, reply, and continue;
- CompleteTask: complete through task-scoped authority and retain terminal
  evidence without returning to ring 3.

Immediate returning calls stay on the same running task. Scheduling boundaries
return to the outer loop. The Worker queue drains first; the existing Verifier
admission token then queues its task, and the same runtime loop drains it.

## Authority

Task mutation uses the delegated capability stored only in
`AgentCallContext`; ring 3 never supplies a capability ID. Mailbox operations
use the authenticated Agent identity and core mailbox ownership rules.

InspectTaskResult and VerifyTask require a separate attenuated Verify
capability. The boot adapter registers that trusted supplemental authority for
the Verifier Agent. The runtime resolves it by authenticated caller identity;
the request supplies only the target Task, and the core remains responsible for
resource-scoped authorization. Workers have no supplemental Verify authority.

## Evidence And Compatibility

QEMU adds:

`AGENT_KERNEL_NATIVE_RUNTIME_LOOP_OK`

It is emitted only after the generic loop has:

- processed all five Worker A calls, five Worker B calls, and four Verifier
  calls;
- crossed Prepared, Preempted, WaitingMailbox, and Yielded physical states;
- matched all three transcripts to their Capsule return offsets and nonces;
- completed all tasks with an empty run queue and empty runtime registry.

All existing success markers remain required. Capsule bytes, digests, call
offsets, semantic order, and the exact 84-event trace remain unchanged.

## Failure And Atomicity

- Invalid decode, identity, nonce, authority, payload, or transcript capacity
  fails before semantic mutation.
- A semantic failure leaves the current call frame unacknowledged.
- A reply is encoded only after the matching syscall and event/state checks.
- Waiting parks the original Receive frame; Yield parks an acknowledged frame.
- Dispatch readiness failure occurs before commit and preserves registry
  ownership.
- Any impossible post-commit physical mismatch is a deterministic boot
  fail-stop, not a silent fallback to caller-selected execution.

## Validation

- Red host tests define transcript ordering, capacity, and mismatch atomicity.
- Host ABI tests continue to prove canonical request and reply registers.
- Full workspace tests and the Supervisor remain green.
- no_std bare-metal check and scoped Clippy pass.
- Debug and release QEMU require the new marker and exactly 84 events.
- Release disassembly continues to prove CR3 switching, complete frame save,
  and `iretq` restoration.

## Non-Goals

V0 does not add new ABI operations, dynamic Agent admission, heap allocation,
unbounded transcripts, multiple tasks per Agent, error replies to ring 3, SMP,
context migration, page-table reclamation, hardware-fault recovery, or
asynchronous external I/O completion. The PIT still proves admission
preemption before each Capsule call sequence; re-arming a quantum between
arbitrary returning calls is the next runtime milestone.
