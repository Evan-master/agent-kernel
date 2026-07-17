# X86 Native Fault Handler Agent V0 Design

## Status

Implemented and validated on 2026-07-17; publication is pending.

## Purpose

Demand Page V1 proves that the kernel can repair one retained not-present page,
but bootstrap still chooses when that repair is allowed. Native Fault Handler
Agent V0 moves that policy decision into a real, isolated ring-3 Agent. The
kernel records and routes the fault, the Handler consumes the structured fault
message and submits an auditable decision, and only then may bootstrap exercise
its existing Rollback authority to perform the bounded PTE activation.

The proof path becomes:

```text
Fault Worker #PF(6)
    -> kernel FaultRecord
    -> installed RouteToHandler policy
    -> blocking mailbox wake
    -> ring-3 Fault Handler Agent
    -> acknowledged fault message + TaskResult approval
    -> exact retained-page activation
    -> same-frame Fault Worker resume
```

## Layer Placement

- `agent-kernel-core` owns the first-class FaultHandler image/entry role and
  atomic integration between fault routing and an existing mailbox waiter.
- `agent-kernel` remains a syscall facade; it receives no privileged shortcut.
- `agent-kernel-boot` forwards explicit fixed capacities for fault handlers and
  policies into the facade type.
- The host-testable x86 library extends the receive reply with bounded fault
  payload fields and recognizes a FaultHandler Capsule kind.
- The bare-metal x86 adapter owns the immutable Handler Capsule, its private
  address space and CPU frame, boot admission, exact runtime transcript, and
  terminal QEMU evidence.

## First-Class Handler Role

`AgentImageKind` and `AgentEntryKind` gain `FaultHandler`. A FaultHandler image
may launch only through a matching FaultHandler entry. Capsule header kind `3`
is reserved for this role; Worker and Verifier kinds remain `1` and `2`.

The boot proof uses Agent 7 and a separately delegated task, capability, image,
address-space root, code frame, signal frame, four stack frames, and retained
lazy frame. Its physical memory identity must be pairwise disjoint from all
four existing native task Agents.

## Structured Fault Message ABI

The existing ReceiveMessage reply remains operation 8 and keeps its current
fields:

```text
r10 message id
r11 sender Agent id
r12 MessageKind
r13 payload task id or zero
```

V0 defines the remaining reply words for the routed-fault subset:

```text
r14 payload resource id or zero
r15 payload fault id or zero
rbp payload intent id or zero
```

Capability and action payload fields remain unsupported and fail closed. Every
present ID must be non-zero. Existing task-only messages retain zero in the new
reply words, so the extension is backward compatible within Agent Call ABI v1.

## Atomic Waiter Wake

`route_fault_to_handler` and the `RouteToHandler` branch of
`apply_fault_policy` must detect an active mailbox waiter for the selected
Handler. Before any mutation they reserve message, event, and run-queue
capacity and revalidate that the waiting Agent is admitted for its task.

With no waiter, existing ordering remains unchanged. With a waiter, ordering is:

```text
MessageSent
MessageWaitWoken
FaultRouted
FaultPolicyApplied  // policy path only
```

The wake changes the Handler task from Waiting to Accepted, clears its semantic
execution context to Idle, deactivates the waiter, and queues exactly that task.
Any capacity or state failure leaves message, waiter, task, queue, policy, fault,
and event state unchanged.

## Handler Capsule Decision

Before its first Agent Call, the immutable Handler Capsule waits for physical
quantum generation 1. This forces one real PIT preemption and redispatch before
the Handler can enter the call protocol. It then executes exactly five
authenticated calls:

1. DescribeContext.
2. ReceiveMessage, initially blocking with no polling.
3. AcknowledgeMessage for the delivered fault message.
4. SubmitTaskResult.
5. CompleteTask.

The proof Capsule accepts only message 2 from bootstrap Agent 1 with kind
Fault, resource 1, Fault Worker task 4, intent 4, and lazy-page fault 4. Any
other register value loops without producing authority. Its approval result is:

```text
code  = 0xf001  // activate retained demand page
value = 4       // exact FaultId being approved
```

The canonical kind-3 Capsule is 186 bytes: a 32-byte header and 154 bytes of
code. It uses nonce `0xe55ce005`, return offsets `45/54/122/143/152`, and SHA-256
digest `e8ff62e3a146c0a67ca4a8b60523c6e1008968ec75727fdc4835b2274db7c596`.

Bootstrap validates the completed physical transcript, authenticated Handler
context, nonce, return RIPs, acknowledged message record, exact TaskResult,
still-faulted target task, and unchanged #PF(6) detail before receiving an
opaque approval token. The repair path requires that token and matches its
FaultId before touching the PTE.

The Handler does not receive the Fault Worker's Rollback capability and cannot
write page tables directly. It selects policy; the deterministic kernel adapter
executes the already bounded mechanism under explicit bootstrap authority.

## Runtime And Event Proof

The Handler first runs to a native blocking ReceiveMessage state. The first
three Fault Worker traps retain their existing restart behavior. Only the final
not-present #PF(6) applies the installed policy and wakes the Handler.

Relative to Demand Page V1, setup adds ten Handler task/image events plus
`FaultHandlerInstalled` and `FaultPolicyInstalled` at events 59 through 70.
Runtime adds Handler queue/dispatch/quantum-expiry/redispatch/wait at events 91
through 95, the four-event policy route and wake at events 127 through 130, and
Handler dispatch/receive/acknowledge/result/completion at events 131 through
135. Recovery and same-frame completion remain events 136 through 139; the
Driver proof occupies events 140 through 149.

The terminal trace is exactly 149 events and proves twenty-one dispatches,
eight prepared contexts, two waiting boundaries, nine preempted contexts, one
repaired-fault resume, nine physical quantum expiries, four Agent faults, and
five completed native contexts.

The terminal marker is:

```text
AGENT_KERNEL_NATIVE_FAULT_HANDLER_AGENT_OK
```

## Failure Policy

Boot fails closed on a mismatched Handler role, image digest, private memory
identity, context, fault payload, message status, waiter state, event order,
TaskResult, transcript, FaultId, target fault detail, approval token, runtime
counter, PTE state, resumed byte, or final queue state. Kernel-origin faults
remain fatal.

## Validation

- Core red/green tests cover direct and policy fault-route wake ordering plus
  atomic capacity failure.
- ABI tests cover structured fault receive replies and rejection of unsupported
  or zero-valued payload fields.
- Image and boot tests cover the first-class role and fixed-capacity forwarding.
- Formatting, full workspace tests, Supervisor output, no_std checks, and
  warnings-denied scoped Clippy remain green.
- Debug and release QEMU require the new markers and exactly 149 events.
- Release disassembly confirms saved-CS CPL selection, CR2 capture before the
  CR3 restore, error/RIP offsets `0x78/0x80`, the full-register `iretq` resume,
  and the exact embedded Handler Capsule call sequence and digest.

## Non-Goals

V0 does not provide a general pager, arbitrary Agent page-table writes,
fault-time allocation, multiple handlers per key, handler chaining, wildcard
policies, dynamic Handler code, model calls in kernel space, denial/retry
results, multiple outstanding faults, swapping, eviction, copy-on-write,
unmapping, PCID, SMP TLB shootdown, or recovery of any fault other than the one
existing retained lazy page.
