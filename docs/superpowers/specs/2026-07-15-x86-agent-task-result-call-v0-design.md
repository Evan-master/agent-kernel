# X86 Agent Task Result Call V0 Design

## Status

Implemented and validated on 2026-07-15.

## Purpose

The native Agent Call path can now complete a running task, but completion is a
terminal mutation and the task carries no Agent-produced result. This milestone
adds a fixed-width kernel-visible TaskResult and lets each ring-3 Worker submit
one result, receive a successful reply, continue execution, and then request
CompleteTask.

This is the first returning Agent Call with a semantic side effect. It proves
that the architecture boundary can suspend user code, authorize and record a
domain mutation, construct a bounded reply, and safely resume the same physical
Agent context without exposing capability IDs or accepting pointers.

## Core Task Result

`TaskResult` is an allocator-free value:

```rust
pub struct TaskResult {
    pub code: u16,
    pub value: u64,
}
```

`code` is a compact result schema/outcome tag interpreted by a future verifier;
`value` is the corresponding fixed-width value or content commitment. TaskResult
does not itself mean verified success. Completion and verification remain
separate lifecycle transitions.

Each Task stores `result: Option<TaskResult>`, initially `None`. A successful
submission stores the value and emits exactly one `TaskResultSubmitted` event
carrying the same result. The result remains queryable through Completed and
Verified states.

## Authorization And Atomicity

`submit_task_result(agent, capability, task, result)` requires:

1. an active Agent,
2. an existing Task,
3. active `Act` authority scoped to exactly that Task,
4. Task status `Running`,
5. the caller to be the assignee,
6. a matching admitted Worker launch entry,
7. no existing TaskResult,
8. one available event slot.

Failure leaves the Task, execution context, run queue, event sequence, and
result unchanged. A successful submission does not complete, yield, requeue, or
otherwise alter scheduler state. Duplicate submission returns
`TaskResultAlreadySubmitted`.

Completion does not require a result in V0 so existing kernel callers remain
compatible. Requiring a result can later be expressed by intent or verification
policy instead of silently changing every task contract.

## ABI Extension

Agent Call ABI version 1 adds operation `4` = SubmitTaskResult:

| Register | Value |
| --- | --- |
| `RAX` | `AGNTCALL` magic |
| `RBX` | ABI version `1` |
| `RCX` | SubmitTaskResult operation `4` |
| `RDX` | flags `0` |
| `RSI` | Agent ID returned by DescribeContext |
| `RDI` | Task ID returned by DescribeContext |
| `R8` | Agent Image ID returned by DescribeContext |
| `R9` | DescribeContext nonce |
| `R10` | canonical zero-extended `u16` result code |
| `R11` | result value |

DescribeContext, Yield, and CompleteTask continue to require `R10/R11 == 0`.
SubmitTaskResult requires the four context words and nonce to match the trusted
scheduler context. Values above `u16::MAX` in R10 are noncanonical and rejected.

On success, the kernel reply uses the same magic/version/status envelope,
identifies operation 4 in RDX, returns the trusted Agent/Task/Image/nonce in
RSI/RDI/R8/R9, and clears R10/R11. The reply contains no capability. The Worker
then changes only RCX to CompleteTask, clears RDX, and makes its terminal call.

## CPU Type State

The physical call flow becomes:

`PreemptedAgentCpu -> RequestedTaskResultCpu -> AcknowledgedTaskResultCpu -> CompletedAgentCpu`

`RequestedTaskResultCpu` owns the captured frame and trusted call context but
does not imply a semantic mutation. The timer/task adapter compares that context
with its kernel-owned WorkerTask, calls `sys_submit_task_result`, validates the
event and unchanged Running state, then encodes the success reply and returns an
`AcknowledgedTaskResultCpu`.

Only that acknowledged type can resume to the terminal CompleteTask call. Each
Worker therefore performs three physical calls and six Agent/kernel CR3
transitions. The CPU layer continues to decode frames and manage address-space
mechanics only; it does not call task syscalls.

## Boot Proof And Events

Worker A submits `{ code: 0x0a01, value: 0xa11c0001 }`; Worker B submits
`{ code: 0x0b02, value: 0xb22c0002 }`. Their values, images, nonces, physical
frames, and CR3 roots remain distinct.

The event count grows from 55 to 57:

1. Events 1 through 42 are unchanged.
2. A result submission is event 43 and A completion is event 44.
3. B dispatch is event 45.
4. B result submission is event 46 and B completion is event 47.
5. Existing UART/Driver events shift to 48 through 57 without semantic change.

After event 47 both Tasks retain their different results in Completed state,
both execution contexts are Idle, and the run queue is empty.

The shorter post-Describe code sequence gives Worker A return offsets 46, 67,
and 76; Worker B retains its two-NOP prefix at 48, 69, and 78. Both Capsule
digests change and remain bound before dispatch.

## Validation

Core tests cover successful persistence/event replay, unchanged Running state,
duplicate rejection, exact task scope, invalid status, and event-capacity
atomicity. Facade tests cover the syscall boundary. ABI tests cover operation 4,
canonical R10, context/result matching, result reply encoding, and legacy
reserved-register rules.

QEMU must prove both returning mutations, both terminal completions, six CR3
transitions per Worker, distinct results and return offsets, exactly 57 events,
and unchanged Driver terminal behavior. Release disassembly must still show
full frame capture, kernel CR3 restoration, and no assembly-level operation
classification.

The completed implementation passed all workspace tests, both host and bare
x86 scoped Clippy with warnings denied, debug QEMU, and release QEMU. Each QEMU
run emitted both result markers, events 43 and 46, exactly 57 ordered events,
and `SUPERVISOR_HANDOFF_READY`. Release symbols place enter, resume, timer, and
call boundaries at `0x105ea`, `0x10641`, `0x10675`, and `0x106e8`; disassembly
shows all 15 integer registers captured before CR3 restoration and no semantic
operation branch in either assembly stub.

Full-workspace Clippy remains blocked by the same eight pre-existing
`too_many_arguments` findings in unchanged core modules. None is in this
milestone's new result store, ABI, CPU type-state, or task adapter code.

## Non-goals

V0 does not accept pointers or variable payloads, allocate a result store,
interpret application result codes, require every task to submit a result,
verify the result, fulfill the intent, expose capabilities to ring 3, return
recoverable authorization errors to Agent code, support repeated/progressive
results, destroy completed address spaces, or add SMP.
