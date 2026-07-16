# X86 Native Verifier Agent V0 Design

## Status

Implemented and validated on 2026-07-16.

## Purpose

Workers can now submit fixed-width results and complete, but verification still
comes only from trusted host or boot code. This milestone introduces a native
Verifier image and runs it as its own scheduled task under a third isolated
ring-3 address space.

The Verifier inspects Worker A's result through an authorized returning Agent
Call, compares the returned words in its own machine code, verifies Worker A,
receives that mutation's success reply, and finally completes its own task.
Worker B remains Completed as a control proving that verification is scoped to
one selected result.

## First-Class Verifier Identity

Core adds `AgentImageKind::Verifier` and `AgentEntryKind::Verifier`. The launch
image and entry kinds must match exactly. The x86 Capsule format assigns image
kind `2` to Verifier while retaining kind `1` for Worker; all other kinds remain
unsupported by the native loader.

The Verifier owns two independent capabilities:

1. a task-scoped Act capability that admits and completes its own scheduled
   verification task,
2. a resource-scoped Verify capability used only to inspect and verify the
   selected Worker task.

Neither capability appears in ring-3 registers. The scheduler-owned call
context contains only the private launch capability; the verification adapter
retains the separate Verify capability.

## Audited Result Inspection

Core adds:

```rust
inspect_task_result(agent, capability, task) -> Result<Event, KernelError>
```

The operation requires an active Agent, resource-scoped Verify authority, a
Completed target with a stored result, and a currently Running task admitted by
a Verifier launch entry on the same resource. Missing results return
`TaskResultMissing`.

Success emits `TaskResultInspected` with the exact TaskResult and otherwise
leaves the target, Verifier task, execution contexts, and run queue unchanged.
The event makes the read auditable and gives the architecture adapter one
bounded value to return. Capacity failure is atomic.

Existing `verify_task` remains compatible for supervisor callers. The physical
Verifier path can reach it only after a successful inspection type state, then
validates both `TaskVerified` and `IntentFulfilled` before constructing a
success reply.

## Agent Call ABI

ABI version 1 adds operations 5 and 6:

| Register | InspectTaskResult request | VerifyTask request |
| --- | --- | --- |
| `RAX` | `AGNTCALL` | `AGNTCALL` |
| `RBX` | version 1 | version 1 |
| `RCX` | operation 5 | operation 6 |
| `RDX` | flags 0 | flags 0 |
| `RSI` | Verifier Agent ID | Verifier Agent ID |
| `RDI` | Verifier execution Task ID | Verifier execution Task ID |
| `R8` | Verifier Image ID | Verifier Image ID |
| `R9` | DescribeContext nonce | DescribeContext nonce |
| `R10` | target Worker Task ID | target Worker Task ID |
| `R11` | zero | zero |

Both requests must echo the trusted execution context and the one kernel-chosen
target task. Inspect success returns operation 5 in RDX and the inspected result
in R10/R11. Verify success returns operation 6 and clears R10/R11. Both replies
preserve trusted identity and nonce. Legacy operations retain their prior
register contracts.

## CPU Type State

The physical Verifier sequence is:

`PreemptedAgentCpu -> RequestedTaskInspectionCpu -> AcknowledgedTaskInspectionCpu -> RequestedTaskVerificationCpu -> AcknowledgedTaskVerificationCpu -> CompletedVerifierCpu`

The CPU layer captures and decodes frames only. The Verifier task adapter:

1. binds the inspection request to its subject and calls the inspection syscall,
2. acknowledges with the event's result only after scheduler state is unchanged,
3. binds the next request to the same target and calls `sys_verify_task`,
4. acknowledges only after target status, result, and intent events are valid,
5. accepts the final CompleteTask request only for the Verifier's own task.

The Verifier performs four Agent Calls and eight Agent/kernel CR3 switches.

## Native Capsule

The Verifier image uses nonce `0xc33ce003` and targets Worker task 1. After the
inspection reply it compares R10 with `0x0a01` and R11 with `0xa11c0001`; either
mismatch jumps to a terminal loop before VerifyTask. Reaching the captured
VerifyTask call therefore proves the comparison ran at CPL3.

Expected return offsets are 46 for DescribeContext, 64 for InspectTaskResult,
100 for VerifyTask, and 109 for CompleteTask. The code is 111 bytes and the
Capsule is 143 bytes. Its SHA-256 digest is bound to a verified Verifier image
record before dispatch.

## Schedule And Events

Verifier setup occurs after both Workers are admitted but before Worker A
dispatches. It registers Agent 5, creates and delegates task 3, derives the
separate Verify capability, verifies the Verifier image, launches the task entry,
and accepts the task without initially queuing it.

The expected 76-event sequence is:

1. Events 1 through 37 remain the existing boot, Driver, and Worker setup.
2. Verifier registration and admission occupy events 38 through 48.
3. Worker scheduling, results, and completion occupy events 49 through 58.
4. Verifier queue, dispatch, expiry, and redispatch occupy events 59 through 62.
5. Result inspection is event 63.
6. Worker A verification and intent fulfillment are events 64 and 65.
7. Verifier task completion is event 66.
8. Existing UART and Driver events shift to 67 through 76.

At handoff Worker A is Verified with its result and fulfilled intent. Worker B
is Completed with its different result and bound intent. The Verifier's own task
is Completed, all three task execution contexts are Idle, and the run queue is
empty.

## Validation

Core tests cover Verifier admission, exact Verify authority, target status,
missing result, replayable inspection, unchanged scheduler state, and event
capacity. Image tests cover kinds 1 and 2 and metadata binding. ABI tests cover
both new operations, target matching, result reply encoding, and legacy rules.

QEMU must prove the third private address space, Verifier PIT preemption, CPL3
result comparison, returning inspection and verification calls, target-only
verification, Verifier completion, exactly 76 events, and unchanged physical
Driver behavior. Debug and release builds plus release disassembly remain
required.

## Non-Goals

V0 does not verify Worker B, execute model inference in kernel space, interpret
arbitrary result schemas, accept pointers or variable payloads, reject a result,
retry verification, verify the Verifier's own completed task, reclaim address
spaces, add SMP, or change supervisor compatibility paths.
