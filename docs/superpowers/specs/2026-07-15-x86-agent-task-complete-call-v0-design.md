# X86 Agent Task Completion Call V0 Design

## Status

Accepted for autonomous implementation on 2026-07-15.

## Purpose

The versioned Agent Call ABI can return trusted context and validate a
cooperative Yield, but the boot adapter still initiates every domain mutation.
This milestone lets ring-3 Worker code request the first capability-authorized
kernel lifecycle transition itself: completing its currently bound task.

The operation is deliberately task-specific. A delegated Worker owns a
task-scoped `Act` capability; core authorization rejects that capability for a
generic action but accepts it for `complete_task` on exactly the scoped task.
The architecture adapter preserves this distinction rather than broadening
authority for demonstration purposes.

## ABI Extension

Agent Call ABI version 1 adds operation `3` = CompleteTask. The common request
header and reserved-register rules remain unchanged. CompleteTask carries the
same context echo as Yield:

| Register | Value |
| --- | --- |
| `RAX` | `AGNTCALL` magic |
| `RBX` | ABI version `1` |
| `RCX` | CompleteTask operation `3` |
| `RDX` | flags `0` |
| `RSI` | Agent ID returned by DescribeContext |
| `RDI` | Task ID returned by DescribeContext |
| `R8` | Agent Image ID returned by DescribeContext |
| `R9` | DescribeContext nonce |
| `R10`, `R11` | zero |

All four payload words must match the immediately preceding trusted reply.
Unknown or malformed calls fail before any task mutation.

## Trusted Completion Authority

`AgentCallContext` gains a private `CapabilityId`. DescribeContext does not
return it, and no request register can select or replace it. The context is
constructed only from the admitted `WorkerTask` tuple:

- Agent ID,
- Task ID,
- verified Agent Image ID,
- delegated task capability ID.

Queued-state validation proves the task record owns that delegated capability
and the Agent launch entry uses the same task, image, and capability. A
`CompletedAgentCpu` token retains this trusted context after validating the
physical CompleteTask request.

The semantic adapter compares the token context with its scheduler-owned
`WorkerTask`, then calls:

`sys_complete_task(agent, delegated_capability, task)`

Core remains the final authority. It checks capability chain activity, agent,
resource, `Act`, task scope, running status, assignee, and launch admission
before producing `TaskCompleted`.

## Terminal Scheduling

CompleteTask is terminal for the running physical context. The kernel does not
return a completion reply to code whose task is now `Completed`. After Worker A
completes, its execution context must be `Idle`; the scheduler dispatches B.
After B completes, both tasks are terminal, both execution contexts are idle,
and the run queue is empty.

The event count remains 55:

1. Events 1 through 42 retain boot, image, admission, and timer rotation.
2. A completion is event 43.
3. B dispatch is event 44.
4. B completion is event 45.
5. Existing UART/Driver flow remains events 46 through 55.

No synthetic Yield is recorded. The event log therefore reflects the operation
the Agent actually requested.

## CPU Evidence

Each Worker still performs two Agent calls and four Agent/kernel CR3
transitions: returning DescribeContext, then terminal CompleteTask. The runtime
captures the exact return RIP of both call instructions, verifies one call per
mailbox reset, and exposes no completion token unless operation, context echo,
nonce, frame, CR3, selectors, flags, RSP0 bounds, and canary all match.

Worker A retains call return offsets 46 and 70. Worker B retains offsets 48 and
72 behind its two-NOP prefix. Changing the opcode changes both Capsule digests,
which remain verified and read back before dispatch.

## Failure And Event Policy

Malformed or mismatched physical calls produce no semantic event. Kernel
authorization, state, capacity, or event-log failure also leaves task state
unchanged under the existing core atomicity guarantees. Boot stops before
success markers if either layer rejects completion.

Successful completion is a domain mutation and must emit exactly one
`TaskCompleted` event. DescribeContext remains a read-only architecture call and
emits no semantic event.

## Validation

Host contracts add CompleteTask parsing, context/capability construction, and
wrong-context rejection. Existing core tests continue to prove a task-scoped
capability cannot authorize generic action or another task's completion. QEMU
must prove both physical completion calls, delegated authority use, terminal
task/context/queue state, unchanged Driver semantics, and exactly 55 events.

Release disassembly must retain full call-frame capture, kernel CR3 restoration,
and absence of assembly-level operation classification.

## Implementation Evidence

Development began from two observed failures:

- the focused host contract could not resolve the absent CompleteTask constant,
  request variant, context capability, and matcher,
- the unchanged QEMU image reached its prior Yield flow but failed because the
  authority marker and TaskCompleted events were absent.

The final implementation passes:

- nine focused ABI contracts covering CompleteTask constants, canonical decode,
  zero payload rejection, all context/nonce mismatches, private authority
  binding, DescribeContext replies, and the retained Yield ABI contract,
- every workspace test, including the existing proofs that task-scoped
  authority cannot authorize generic Act or another task's completion,
- the complete 78-event host supervisor scenario,
- the bare `x86_64-unknown-none` target check,
- host/all-target and bare-target Clippy for `agent-kernel-x86_64` with
  dependencies excluded and warnings denied,
- debug and release QEMU with both authority/completion markers, events 43 and
  45 as TaskCompleted, exactly 55 ordered events, terminal task/context/queue
  state, and unchanged Driver completion.

Repository-wide Clippy with dependencies included remains blocked by eight
pre-existing `too_many_arguments` findings in unchanged core modules. This is
recorded baseline debt rather than a completion-call regression.

The final Capsule digests are
`3fe1dbd6a29bc653bb5c725ffc58cb114241daa87ee7e16ef433de8498c71256`
for Worker A and
`9d68ddbbd51949b779014f9f03a7eb4b8d2f2dda8641b88afd2a524e86f4b9a7`
for Worker B. Release symbols place `agent_kernel_enter_user` at `0x14511`,
`agent_kernel_resume_interrupted_user` at `0x14568`, the timer stub at
`0x1459c`, and the Agent-call stub at `0x1460f`. Disassembly shows both Agent
return paths selecting Agent CR3 before `iretq`; the call stub saves all 15
integer registers, restores kernel CR3, records only CR3/RSP/RIP/count/seen
evidence, and returns without classifying the requested operation.

## Non-goals

V0 does not return from a completed task, verify the completed task or fulfill
its intent, expose capability IDs to Agent code, add generic Act/Observe calls,
support arbitrary call dispatch, transfer authority, copy pointers, recover a
failed call in user code, dynamically destroy address spaces, or add SMP.
