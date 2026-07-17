# Agent Kernel

Agent Kernel is an early prototype for an agent-native operating system kernel.
It is not a Linux wrapper, shell agent, or POSIX-first compatibility layer.

The project starts from new OS primitives instead of POSIX compatibility:
agents, owned resources, resource lifecycle, capabilities, capability
attenuation, agent launch entries, runtime admission, typed intents, actions, observations,
agent executable image identity records, checkpoints, rollback, verification,
tasks, fixed-width task results, delegation, native blocking mailbox IPC, task wait signals, task fault traps,
fault handlers, fault policies, memory cells, native object namespace entries,
driver bindings, device events, driver invocations, driver commands, agent
execution contexts, driver endpoint registries, HAL dispatch requests, and
event logs.

## Current Scope

- `agent-kernel-core`: no_std-friendly agent registry, agent image records, agent launch entries, runtime admission, agent execution contexts, owned resource creation, resource lifecycle, capability lifecycle, capability attenuation, action, observation, checkpoint, intent store, task store, fixed-width task results, lifecycle, kernel-selected FIFO run queue, blocking mailbox IPC, task wait signals, task fault traps, fault handlers, fault policies, memory cells, object namespace entries, driver endpoint registry, driver bindings, device event lifecycle, driver invocation scheduling, driver command lifecycle, rollback, and event model.
- `agent-kernel`: no_std kernel facade with syscall-style methods over the core model.
- `agent-kernel-hal`: no_std device backend contract for executing immutable, kernel-authorized driver requests.
- `agent-kernel-boot`: no_std boot handoff boundary that seeds the kernel with a deterministic bootstrap flow, explicit owner rollback authority, and trusted mutable architecture initialization.
- `agent-kernel-x86_64`: no_std x86_64 bootloader entry, SHA-256-bound one-page Worker, Verifier, and FaultHandler Capsules, five isolated Agent CR3 roots with same-address private pages, a fixed-capacity prepared/preempted/mailbox-waiting/yielded/fault-repaired CPU ownership registry consumed by a kernel-selected dispatch-and-take runtime loop, owned suspended CPU frames, PIT IRQ0 quanta re-armed before every ring-3 entry or resume through a shared RSP0 stack, strict AgentCall/QuantumExpired/AgentFault physical boundary classification, CPL-aware #UD/#GP/#PF containment into kernel `ExecutionTrap` state, three consuming fault-to-fresh-entry restarts with signal/stack/lazy-frame scrubbing, and one not-present page repair gated by a native ring-3 Fault Handler decision plus bootstrap Rollback authority. It also provides a read-only per-Agent call-release/quantum/restart signal page, a versioned returning Agent Call ABI whose role-independent inner loop routes all nine v1 operations and records fixed-capacity call transcripts, cooperative Yield, blocking mailbox wait/wake plus Send/Receive/Acknowledge, audited task-result submission and inspection, target-scoped verification, task completion, one-shot UART IRQ4 ingress, and byte-wide Port I/O behind the privileged Driver boundary.
- `agent-kernel-image`: host-side BIOS image builder and QEMU argument helper.
- `agent-supervisor`: host-side user-space simulator that drives the prototype and executes a stateful virtual register device backend.

## Current Behavior

The v0 flow is deliberately small:

1. Register the owner, target, and fault handler agents with idle execution contexts.
2. Register a workspace resource.
3. Grant the owner agent a capability for observe, act, verify, checkpoint, rollback, and delegation.
4. Register and verify a supervisor executable image, then launch the owner agent into a resource-scoped supervisor entry.
5. Install a resource-scoped task fault handler and fault policy.
6. Record the capability grant, image registration, image verification, launch, and fault automation setup in the kernel event log.
7. Observe the resource and store an observation record.
8. Execute an action with an ActionId and store an action record.
9. Request verification for that action.
10. Create and store a checkpoint record.
11. Request rollback for that checkpoint.
12. Declare a typed action intent that requires verification.
13. Create a kernel-owned task from that intent.
14. Bind the intent to the task.
15. Delegate the task to another agent.
16. Record the derived task-scoped capability in the kernel event log.
17. Register and verify a worker executable image, then launch the assignee into a task-scoped worker entry using that derived capability.
18. Let the assignee accept the task.
19. Enqueue the accepted task and dispatch it into `Running` state with a deterministic quantum.
20. Advance the running task by one explicit scheduler tick.
21. Advance it again so the quantum expires and the task is requeued.
22. Redispatch the accepted task from the run queue.
23. Trap the running task into a kernel fault record.
24. Apply the installed fault policy to route the fault to the handler.
25. Let the handler receive and acknowledge the fault message.
26. Recover, requeue, and redispatch the faulted task.
27. Let the assignee wait on a typed workspace signal.
28. Emit the signal and wake the waiting task back into the run queue.
29. Redispatch the woken task.
30. Let the assignee complete the running task.
31. Request verification for the completed task.
32. Mark the intent fulfilled after task verification.
33. Send a native kernel message from the owner agent to the target agent.
34. Let the target agent receive and acknowledge that message.
35. Create a native memory cell under a memory resource.
36. Recall and remember memory cell state through explicit capabilities.
37. Bind a memory cell into the workspace object namespace.
38. Resolve that namespace entry through an observe capability.
39. Rebind the namespace entry to the created task.
40. Create an owned temporary service resource under the workspace.
41. Retire that service resource through its owner capability.
42. Derive an observe-only capability from the owner to the target agent.
43. Let the target agent observe the workspace through that derived capability.
44. Register a dedicated Driver Agent with its own idle execution context.
45. Create an owned device resource under the workspace.
46. Register an immutable virtual endpoint for the device under `Delegate` authority.
47. Derive observe/act driver authority from the owner to the Driver Agent.
48. Register and verify a Driver image, then launch the agent into a device-scoped Driver entry.
49. Bind the Driver Agent as the native driver for that device.
50. Raise a typed device event against the bound device.
51. Deliver that event and atomically queue a Driver Invocation.
52. Dispatch the invocation with a deterministic quantum and advance one explicit tick.
53. Let the running Driver Agent acknowledge the event.
54. Let it submit a typed command causally linked to the running invocation.
55. Resolve its kernel-owned endpoint and dispatch an immutable request to the HAL boundary.
56. Execute the request against a stateful virtual device and record its fixed-width result.
57. Complete the invocation and return the Driver Agent execution context to idle.
58. Print the kernel event log from the supervisor.

All resource operations go through explicit capabilities. Agent registration,
agent launch, agent image registration, agent image verification, agent image retirement, agent
suspension, agent resume, agent retirement, owned resource creation,
capability grants, derived root capabilities, derived task capabilities, typed
intent declarations, action, verification, checkpoint creation, rollback
requests, task creation, intent binding, task completion, task verification,
intent fulfillment, delegation, scheduler ticks, quantum expiry, task fault
trapping, fault handler installation, fault policy installation, fault routing,
fault policy application, task fault recovery, task waiting, signal emission,
task wakeup, message send, mailbox wait start, mailbox wakeup, message receive,
message acknowledgement, memory cell creation, memory recall, memory remember,
namespace bind, namespace resolve, namespace rebind, resource retirement, driver
endpoint registration, driver binding, device event raise, device event delivery, device event
acknowledgement, driver invocation queueing, dispatch, ticks, quantum expiry,
and completion, plus driver command submission, dispatch, completion, and
failure are first-class kernel events, not external tooling.
Agents, agent images, launch entries, resources, checkpoints, waiters, fault records, fault
handlers, fault policies, messages, memory cells, namespace entries, driver
endpoints, driver bindings, device events, driver invocations, and driver commands are also
queryable fixed-capacity kernel records, and new root or derived capabilities
can only be issued to active registered agents. Kernel operations that act on
behalf of an `AgentId` reject unknown, suspended, or retired actors before authorization, state, queue,
mailbox, memory, or capacity checks. Each registered agent has a fixed-capacity
execution context that tracks whether the agent is idle, running a task or
Driver Invocation, waiting on a signal or mailbox receive, or faulted on a task. Agent images store kernel-owned
executable identity metadata: digest, kind, ABI version, entry version, owner,
resource, and pending/verified/retired status. Agent images are registered as
pending executable identities. A verifier-capable agent must verify the image
before launch records can reference it. Resource-scoped launch entries bind an
active agent to a resource, `Act` capability, verified Agent Image, entry kind,
and optional declared action intent. Task-scoped launch entries bind a worker
to one delegated task, its derived task capability, and a verified worker Agent
Image. Runtime mutation paths require
an admitted launch entry before queue, dispatch, tick, yield, wait, fault,
completion, or signal wakeup state can change. Dispatch refuses to run a second
task for an agent whose context is already busy. Rollback moves the checkpoint into
`RollbackRequested` status. Accepted tasks move through a fixed-capacity FIFO
run queue, become `Running` with an explicit quantum, accumulate deterministic
ticks, return to the queue when their quantum expires, and can enter `Waiting`
until an authorized signal emission or a message send wakes them back into the
run queue. Kernel-selected dispatch consumes its own FIFO head and returns the
exact Agent/Task identity made `Running`; architecture code no longer supplies
the expected Agent as the scheduling decision. One mutable x86 dispatch-and-take
operation prepares the read-only core permit, finds any matching parked CPU
state for its Agent/Task, commits the semantic transition, and consumes that
exact state. Architecture callers supply neither an Agent identity nor an
expected physical kind. A native Yield parks the complete call frame, appends
the running task behind the existing FIFO head, and resumes that frame only
after a later kernel-selected dispatch.
`IntentId`, `TaskId`, and `MessageId` values are allocated by fixed-capacity kernel stores rather than
invented by the supervisor. `WaiterId` and `MemoryCellId` values are also kernel-allocated, and
memory recall writes an audit event before returning a value. Delegation derives
a task-scoped action capability for the assignee, so the supervisor does not
grant broad resource authority to complete delegated work. Revoking the source
capability that authorized delegation also invalidates the derived task-scoped
capability before future task authorization succeeds. Mailbox IPC stores typed
kernel object references instead of heap-allocated bytes or host transport
handles. An empty authorized receive can atomically retain a task waiter and
move its execution context to `Waiting`; the next send appends the message,
deactivates that waiter, and requeues the task before the original call returns.
Memory cells store fixed-width typed words instead of files, byte
buffers, or host persistence. Namespace entries bind compact keys to typed
kernel object references inside workspace resources rather than parsing paths or
delegating lookup to a host filesystem. Retired resources remain queryable for
audit, but active-resource lookup rejects future grants, child resources, and
old-capability operations against them.
General capability derivation lets an agent attenuate its own authority for
another active agent; the derived capability cannot exceed the source
operations, cannot be created from task-scoped authority, and becomes unusable
when the source capability is revoked.
Driver bindings assign one active agent as the driver for an active `Device`,
`Network`, or `Service` resource. Binding requires explicit `Delegate`
authority and does not mint a capability or launch the driver. Delivery requires
a verified `Driver` image, a device-scoped `Driver` launch entry, and live
`Observe`/`Act` entry authority. It atomically moves the event to `Delivered`
and appends a queued `DriverInvocation`. Invocation dispatch, explicit ticks,
quantum expiry, and completion are kernel-owned transitions that share the
agent execution context with task scheduling. Event acknowledgement and causal
commands require that invocation to be running. The bound driver can also
submit fixed-width commands without an event cause under `Act` authority and
move each command exactly once from `Submitted` through `Dispatched` to
`Completed` or `Failed`. Dispatch returns an immutable request to the no_std HAL
contract. The supervisor's virtual register backend executes that request and
returns the terminal outcome without owning kernel authority or command state.
The kernel endpoint registry maps each device-like `ResourceId` to one validated
`Virtual`, `Mmio`, or `Port` descriptor under `Delegate` authority. It rejects
invalid or overlapping ranges, preserves retired mappings for audit, and blocks
command dispatch until the resource has an endpoint. Raw addresses are never
Agent command data. All records and transitions are replayable. The x86_64
architecture backend now executes byte-wide `Read` and `Write` commands against
bounded `Port` endpoints, treating `opcode` only as a relative offset. Its QEMU
boot path first installs a persistent 256-entry IDT with explicit gates for CPU
exception vectors 0 through 31. A returning breakpoint stub captures and
validates the exact CPU return RIP. Vectors 6, 13, and 14 are later replaced by
CPL-aware Agent gates: kernel-origin #UD/#GP/#PF still reach deterministic fatal
stubs, while ring-3 #UD captures a 160-byte frame, ring-3 #GP captures a
168-byte frame plus its CPU error code, and ring-3 #PF additionally captures
CR2 before returning to trusted kernel context. All remaining exception gates
lead to vector-specific deterministic failure stubs. Before that IDT becomes active,
the kernel installs a permanent GDT with ring-0/ring-3 segments and a long-mode
TSS whose RSP0 points at a dedicated 32 KiB privileged stack. The boot adapter
then registers and verifies two normal Worker images, one Fault Worker image,
one Verifier image, and one first-class FaultHandler image.
Each immutable native Capsule has a fixed 32-byte AgentOS header followed by at
most one code page; the no_std loader rejects unsupported format, architecture,
kind, or flags, zero or mismatched ABI and entry versions, noncanonical lengths,
reserved data, and out-of-range entry offsets. It computes SHA-256 across the
exact header and code bytes and requires equality with the verified kernel image
record before any task can dispatch. A bounded allocator then consumes only
BootInfo `Usable` frames and creates five distinct Agent P4 roots. All five
inherit identical supervisor-only kernel mappings, while each dedicated P4
index 128 contains private frames at the same virtual addresses: one read-only
executable Agent code page, one read-only/NX signal page, an unmapped guard page,
four writable/NX stack pages, and one retained zeroed lazy-data frame whose PTE
starts absent. The kernel CR3 has no translation for any Agent region, the
initial Agent roots do not translate the lazy address, and all
root/code/signal/stack/lazy physical frames are pairwise disjoint.

Each code page is zeroed, filled only from its verified Capsule, and read back
through the supervisor physical alias before mapping. Verifier registration,
task delegation, its separate resource-scoped Verify capability, image
verification, launch, and acceptance occupy events 38 through 48. Fault Worker
registration, task delegation, image verification, launch, and acceptance
occupy events 49 through 58. Fault Handler Agent 7 receives its own delegated
task, capability, kind-3 image, launch entry, handler binding, and
`RouteToHandler` policy at events 59 through 70. The five prepared non-Copy CPU
objects first enter a bounded native runtime registry.
The role-independent outer loop asks the core for its FIFO-head permit, finds a
matching owned CPU state using only the returned Agent/Task identity, commits
the dispatch, and consumes that exact state. Before event 71 this selects Worker
B's prepared CPU without a caller choosing B or a physical state kind. The same
loop later handles preempted, mailbox-waiting, and yielded contexts. A generic
inner loop routes all nine Agent Call ABI v1 operations and records every
operation plus return offset in a fixed-capacity transcript tied to the trusted
DescribeContext nonce.
Every Agent enters its declared offset through a five-word
`iretq` frame at CPL3. Entry clears all general-purpose registers, selects that
Agent's CR3, and only then returns to the Agent. PIT IRQ0 performs a hardware
privilege transition to TSS RSP0; assembly saves all integer registers plus
RIP/CS/RFLAGS/user-RSP/user-SS, records the interrupted CR3, and restores the
kernel CR3 before touching normal kernel context. The kernel validates the
complete 160-byte frame, copies it into the preempted Agent context, and
releases RSP0 for the next Agent. Events 72 through 75 expire B and A in turn,
park each complete preemption frame, and let successive outer-loop iterations
select A's prepared then B's preempted CPU while preserving queue order.
Every later return to ring 3 also starts a fresh PIT quantum. A reset physical
mailbox must classify as exactly one Agent Call, one timer expiry, or one
supported Agent fault; mixed or repeated evidence fails closed. A validated
expiry increments byte 1 of that Agent's read-only signal page without
replacing the public semantic task tick. Byte 2 is reserved for a
kernel-authored restart generation, starts at zero, and is bounded at three.
Nonce, transcript, private memory, and the complete frame remain owned when a
live call session becomes Preempted.

The kernel first releases B's read-only signal through its supervisor alias and
resumes B through DescribeContext and ReceiveMessage. Because its mailbox is
empty, event 76 atomically moves B's task and execution context to `Waiting` and
binds a mailbox waiter to the captured receive frame. The native registry parks
that waiting frame before A's preempted context passes event 77's permit
preflight, dispatch commit, and guarded recovery. Worker A performs five
returning calls:
DescribeContext, SubmitTaskResult, SendMessage, Yield, and CompleteTask. It
sends a Notify carrying its Task ID to Worker B and compares the returned
deterministic Message ID before yielding. Only an exact scheduler-owned
Agent/Task/Image identity and nonce echo can mutate task or mailbox state. A's result is event
78; its send records the message at event 79 and atomically wakes and requeues B
at event 80. After the Send reply, A waits for its read-only physical quantum
generation to advance from 1 to 2. The re-armed PIT captures that live session,
event 81 expires A's second semantic quantum, and the FIFO becomes `[B, A]`
without adding a transcript entry. B redispatches at event 82 only after its
waiting frame matches the prepared permit. The kernel receives Message ID 1 at event 83 before encoding
the reply into B's original saved frame. B then
calls AcknowledgeMessage, SubmitTaskResult, and CompleteTask
at events 84 through 86. Its ring-3 code validates Message ID 1, sender Agent 3,
Notify kind, and Task ID 1; the kernel independently validates the same record.
A redispatches at event 87 only after the same session-preserving preempted
Agent/Task frame passes readiness. It exits the generation wait, validates the
Send reply, yields at event 88, resumes that acknowledged Yield frame at event
89, validates the Yield reply, and completes at event 90. A uses a 148-byte
image with return offsets 46/67/94/131/146. B
uses a 131-byte image with a two-NOP prefix, a different nonce, and return
offsets 48/57/99/120/129. The terminal mailbox record is Acknowledged while both
fixed-width task results remain stored.

After both Workers complete, event 91 queues the Fault Handler task. Its
186-byte kind-3 Capsule waits for physical quantum generation 1 before issuing
an Agent Call, so event 92 dispatches it, event 93 records a real PIT expiry,
and event 94 redispatches the owned preemption frame. It authenticates context
nonce `0xe55ce005`, then its empty ReceiveMessage emits event 95 and parks the
original call frame behind one active mailbox waiter.

The kernel queues the Fault Worker and Verifier at events 96 and 97. The Fault
Worker dispatches first, expires its admission quantum at event 99, and rotates
behind the Verifier. The Verifier dispatches and expires at events 100 and 101,
restoring the FIFO to `[Fault Worker, Verifier]`. Event 102 resumes the Fault
Worker's exact saved frame. Its 116-byte code observes physical quantum
generation 1 and, while restart generation is zero, executes `ud2` at code
offset 42. The vector-6 gate verifies CPL3 origin, saves every integer register
and the privilege frame, switches back to kernel CR3, and classifies the return
as `AgentFault(InvalidOpcode)`. Event 103 records `ExecutionTrap` detail 6 and
leaves only that task and execution context `Faulted`.

The Verifier redispatches after the fault at event 104, proving that one Agent's
invalid instruction did not terminate the kernel or another Agent. Its second
call inspects only Worker A's stored result under resource-scoped Verify
authority and emits `TaskResultInspected` event 105. Ring-3 code compares the
returned words with `0x0a01` and `0xa11c0001`; reaching its third call proves
the match before events 106 and 107 verify Worker A and fulfill its intent. The
fourth call completes the Verifier task at event 108. Worker B remains
Completed but unverified as a target-scoping control.

The physical #UD object is then consumed rather than resumed. The restart path
clears the signal and all four writable stack pages, writes restart generation
1, and drops the saved exception frame. Bootstrap Rollback authority records
recovery and queueing at events 109 and 110. Fresh entry dispatch and PIT expiry
occur at events 111 and 112; event 113 resumes that admission frame before
privileged `cli` traps. The vector-13 gate captures CPU error code zero and
event 114 records `ExecutionTrap` detail 13.

The same consuming restart writes generation 2 and records recovery/queueing at
events 115 and 116. Events 117 through 119 prove another fresh entry, PIT
admission, and redispatch. The Capsule then writes its present read-only signal
page. Vector 14 captures error code 7 and
`CR2=0x0000400000001000`; event 120 records semantic detail
`0xe007400000001000` without changing the signal byte.

The third consuming restart writes generation 3 at events 121 and 122. Events
123 through 125 prove a fourth fresh entry, PIT admission, and redispatch. The
Capsule writes `0x5a` to the absent lazy page at `0x0000400000007000`; vector 14
captures error code 6 and the exact CR2 address, and event 126 records the
fourth immutable fault with detail `0xe006400000007000`.

This final #PF is not repaired immediately. The installed core policy sends
Fault message 2 and atomically wakes Agent 7 at events 127 and 128, then records
the route and policy application at events 129 and 130. Event 131 dispatches
the retained Handler receive frame. The receive reply exposes task 4, resource
1, fault 4, and intent 4 in bounded registers; only after ring-3 code validates
those values does it receive at event 132, acknowledge at event 133, submit
`{code: 0xf001, value: 4}` at event 134, and complete at event 135. The kernel
checks the five-call transcript, return offsets `45/54/122/143/152`, ten CR3
switches, acknowledged message, still-faulted target, and exact event suffix
before producing an opaque approval bound to FaultId 4.

The page repair consumes that approval and independently requires bootstrap
Rollback authority. It maps only the retained zeroed frame user/writable/NX
through the existing 4 KiB page-table leaf, with no fault-time allocation.
Events 136 and 137 record recovery and queueing; event 138 selects the distinct
`RecoveredFault` frame and resumes the unchanged RIP. The retried write and
readback produce `0x5a`, then the Fault Worker authenticates nonce
`0xd44ce004` and completes at event 139 through return offsets 105 and 114. All
four fault records remain in the audit history, all five native task execution
contexts finish Idle, and both semantic and physical queues are empty. This is
an Agent-native image format, call ABI, scheduler, fault/restart/handler
lifecycle, and verification lifecycle, not a POSIX process or syscall ABI.
The PIC is then remapped for IRQ4 and the physical COM1 transmitter-empty
interrupt is armed. A bounded UART top half
captures IIR/LSR state, disables the source, acknowledges the PIC, and returns
through `iretq`. Normal kernel context validates that mailbox, raises an
Interrupt Device Event, and runs its Driver Invocation.
The Driver acknowledges the event, dispatches a causally linked write, records
its terminal result, and completes the invocation. This proves a returning CPU
exception, hardware-enforced ring-3 Agent execution, asynchronous
preemption/resume, a returning Agent call protocol, and one-shot device interrupt
ingress. Multi-page images, writable data segments, relocations, dynamic linking,
signatures, persistent image sources, pointer-bearing calls, asynchronous call
completion, variable hardware quantum lengths, dynamic Agent admission,
page-table teardown, PCIDs, SMP execution, context migration, automatic or
unbounded restart policy, a general pager, multiple lazy pages, fault-time or
replacement-page allocation, arbitrary mapping policy, checkpoint data
restoration, containment of exceptions other than #UD/#GP/#PF,
double-fault IST, a general IRQ
registry, APIC/IOAPIC, MMIO drivers, wider port operations, and DMA policy remain
future work.
Owner-aware resource creation assigns `owner: Some(agent)` and creates the
first capability atomically with the resource. Bootstrap `register_resource`
remains available for system-seeded resources and leaves `owner: None`.

## Boot Handoff

`agent-kernel-boot` currently validates the kernel-native boot contract:

1. Enter kernel phase.
2. On x86_64, install the permanent GDT, long-mode TSS, and dedicated RSP0 stack.
3. Install persistent exception gates and validate an `int3` round trip.
4. Allocate five distinct Agent P4 roots, inherit supervisor kernel mappings, map disjoint fixed CPL3 code, signal, guard, and stack regions at identical virtual addresses, and retain one initially unmapped lazy-data frame per Agent.
5. Initialize `AgentKernel`.
6. Register the bootstrap agent.
7. Register a bootstrap resource.
8. Grant observe/act/verify/delegate/rollback capability to the bootstrap agent.
9. Register a bootstrap executable image as pending.
10. Verify that bootstrap image, moving it from pending to verified.
11. Launch the bootstrap agent into a bootstrap entry that references the verified image.
12. Record observation, action, and verification events.
13. Expose mutable handoff access for trusted architecture initialization.
14. Register a COM1 Port endpoint and admit a dedicated Driver Agent.
15. Register two admitted Worker Agents, create both delegated tasks, and enqueue them without dispatching.
16. Register a Verifier Agent with its own delegated task capability and a separate resource-scoped Verify capability.
17. Register a Fault Worker with its own delegated task and keep it accepted but initially unqueued.
18. Register and verify two normal Worker images, one Fault Worker image, and one Verifier image, binding every launch to its matching image kind.
19. Admit Fault Handler Agent 7 with a delegated task and kind-3 image, then install the exact `ExecutionTrap` handler and `RouteToHandler` policy under bootstrap authority.
20. Parse all five fixed Capsule headers, bind their SHA-256 digests to verified records, copy/read back all five private code pages, and register prepared CPU ownership by trusted Agent identity.
21. Replace IDT vectors 6, 13, and 14 with CPL-aware Agent #UD/#GP/#PF gates while retaining every CPL0 fatal fallback.
22. Let the kernel-selected outer loop dispatch B and A, expire each admission quantum through IRQ0, and park both validated preemption frames.
23. Redispatch B from its preempted state and enter the generic inner loop through DescribeContext.
24. Route B's empty ReceiveMessage into a retained waiting call, then let the outer loop select A's preempted state while B remains `Waiting`.
25. Route A's result submission and typed Notify send, atomically wake B, and capture A's live call session when its re-armed PIT quantum expires.
26. Resume B's retained ReceiveMessage and route its acknowledgement, result, and completion.
27. Redispatch A's session-preserving frame, route Yield, redispatch the acknowledged Yield frame, complete A, and match both Worker transcripts.
28. Dispatch the Fault Handler, force one physical PIT quantum before its first call, redispatch it, and retain its empty ReceiveMessage frame in `Waiting`.
29. Queue Fault Worker before Verifier, expire both admission quanta, and restore FIFO order with Fault Worker first.
30. Resume Fault Worker, capture `ud2` as `AgentFault(InvalidOpcode)`, and commit one `ExecutionTrap` without parking a resumable context.
31. Redispatch Verifier after the fault and route DescribeContext, InspectTaskResult, VerifyTask, and CompleteTask while B remains unverified.
32. Consume the #UD object, scrub its signal and stack pages, set restart generation 1, recover through bootstrap Rollback authority, and requeue a fresh prepared context.
33. Expire that fresh admission quantum, execute privileged `cli`, decode the vector-13 CPU error code, and commit a second immutable `ExecutionTrap`.
34. Consume the #GP object, scrub mutable pages, set restart generation 2, perform a second authorized recovery, and requeue another fresh context.
35. Expire that admission quantum, write the read-only signal page, capture vector-14 error code 7 plus exact CR2, and commit a third immutable `ExecutionTrap`.
36. Consume the protection #PF object, scrub mutable pages, set restart generation 3, perform a third authorized recovery, and requeue a fourth fresh context.
37. Expire that admission quantum, write the absent lazy page, capture vector-14 error code 6 plus exact CR2, and commit the fourth immutable `ExecutionTrap`.
38. Apply the installed policy, atomically send the structured Fault message, wake the blocked Handler, and preserve the target task as `Faulted`.
39. Resume the Handler's retained ReceiveMessage, validate its bounded payload in ring 3, acknowledge it, submit the exact TaskResult approval, and complete the Handler task.
40. Validate the Handler's semantic state, physical transcript, message, result, event suffix, and still-current FaultId before minting an opaque repair approval.
41. Consume that approval under bootstrap Rollback authority, activate the retained zeroed frame user/writable/NX, and queue a distinct repaired-frame context without fault-time allocation.
42. Resume the normalized fault frame at its original RIP, retry the write, prove byte `0x5a`, and complete the Fault Worker while retaining all four fault records.
43. Match terminal evidence for twenty-one kernel-selected dispatches, nine physical quantum expiries, four Agent faults, five completed Idle task contexts, and empty queues.
44. Install IRQ4 in the persistent IDT, remap the PIC, arm COM1 THRE, and receive the hardware interrupt.
45. Validate the interrupt mailbox, execute the causally linked Driver flow, record the write result, complete the invocation, and mark supervisor handoff ready.

The handoff now runs inside QEMU through the x86_64 BIOS image path.

## Non-Goals For V0

- Booting on physical hardware.
- UEFI image support.
- POSIX compatibility.
- Linux syscall compatibility.
- A filesystem, network stack, dynamic/SMP multi-Agent scheduler, or complete physical hardware driver stack.
- Dynamic per-task PIT frequencies or multi-tick hardware quantum lengths.
- Automatic retry/crash-loop policy, more than three fault restarts, a general demand pager, multiple Handler policies or outstanding faults, more than one preallocated lazy page, fault-time frame allocation, replacement address spaces, or containment of CPU exceptions other than ring-3 #UD/#GP/#PF.
- Running an LLM inside kernel space.

## Commands

```bash
PATH="$HOME/.cargo/bin:$PATH" rustup run nightly cargo fmt --check
PATH="$HOME/.cargo/bin:$PATH" RUSTC="$(rustup which rustc --toolchain nightly)" rustup run nightly cargo test --workspace
PATH="$HOME/.cargo/bin:$PATH" RUSTC="$(rustup which rustc --toolchain nightly)" rustup run nightly cargo run -p agent-supervisor
scripts/run-qemu.sh
```

The explicit `PATH` and `RUSTC` prefixes are needed on this local machine
because Homebrew's `cargo` and `rustc` appear earlier in `PATH` than rustup's
toolchain shims.

## QEMU Boot

QEMU is installed through Homebrew:

```bash
brew install qemu
```

Build a BIOS disk image and boot it in QEMU:

```bash
scripts/build-qemu-image.sh
scripts/run-qemu.sh
scripts/run-qemu.sh --release
```

The x86_64 binary requires the `bare-metal` Cargo feature. The image script
enables it automatically; keeping it off by default lets non-x86 host workspace
tests compile only the portable backend library and recording tests.
For `x86_64-unknown-none`, `.cargo/config.toml` selects `sha2`'s documented
compact software backend. The freestanding kernel therefore performs no runtime
CPU-feature dispatch and does not require x86 SHA extensions.

Expected QEMU serial output:

```text
AGENT_KERNEL_QEMU_BOOT_OK
AGENT_KERNEL_GDT_TSS_OK
AGENT_KERNEL_EXCEPTION_BASELINE_OK
AGENT_KERNEL_AGENT_IMAGE_FORMAT_OK
AGENT_KERNEL_AGENT_IMAGE_DIGEST_OK
AGENT_KERNEL_VERIFIER_IMAGE_OK
AGENT_KERNEL_AGENT_USER_MEMORY_OK
AGENT_KERNEL_AGENT_ADDRESS_SPACE_OK
AGENT_KERNEL_VERIFIER_MEMORY_OK
AGENT_KERNEL_FAULT_HANDLER_MEMORY_OK
AGENT_KERNEL_MULTI_AGENT_MEMORY_OK
AGENT_KERNEL_AGENT_IMAGE_LOAD_OK
AGENT_KERNEL_PIT_IRQ_OK
AGENT_KERNEL_AGENT_CPU_PREEMPTION_OK
AGENT_KERNEL_AGENT_RING3_PREEMPTION_OK
AGENT_KERNEL_AGENT_A_PREEMPTION_OK
AGENT_KERNEL_TIMER_PREEMPTION_OK
AGENT_KERNEL_KERNEL_SELECTED_DISPATCH_OK
AGENT_KERNEL_AGENT_CALL_RECEIVE_WAIT_OK
AGENT_KERNEL_NATIVE_BLOCKING_MAILBOX_WAIT_OK
AGENT_KERNEL_AGENT_CALL_SEND_MESSAGE_OK
AGENT_KERNEL_NATIVE_BLOCKING_MAILBOX_WAKE_OK
AGENT_KERNEL_MULTI_AGENT_ISOLATION_OK
AGENT_KERNEL_AGENT_CALL_RECEIVE_MESSAGE_OK
AGENT_KERNEL_AGENT_CALL_ACKNOWLEDGE_MESSAGE_OK
AGENT_KERNEL_AGENT_CPU_RESUME_OK
AGENT_KERNEL_AGENT_CALL_RESULT_OK
AGENT_KERNEL_AGENT_CALL_RETURNING_MUTATION_OK
AGENT_KERNEL_NATIVE_MAILBOX_IPC_OK
AGENT_KERNEL_NATIVE_AGENT_YIELD_OK
AGENT_KERNEL_NATIVE_FAULT_HANDLER_WAIT_OK
AGENT_KERNEL_NATIVE_FAULT_POLICY_ROUTE_OK
AGENT_KERNEL_NATIVE_FAULT_HANDLER_DECISION_OK
AGENT_KERNEL_NATIVE_RUNTIME_STORE_OK
AGENT_KERNEL_VERIFIER_PREEMPTION_OK
AGENT_KERNEL_AGENT_CALL_INSPECT_RESULT_OK
AGENT_KERNEL_AGENT_CALL_VERIFY_OK
AGENT_KERNEL_RESUMABLE_RUNTIME_REGISTRY_OK
AGENT_KERNEL_DISPATCH_READINESS_HANDOFF_OK
AGENT_KERNEL_NATIVE_AGENT_FAULT_CONTAINMENT_OK
AGENT_KERNEL_NATIVE_AGENT_FAULT_RESTART_OK
AGENT_KERNEL_NATIVE_AGENT_GENERAL_PROTECTION_OK
AGENT_KERNEL_NATIVE_AGENT_PAGE_FAULT_OK
AGENT_KERNEL_NATIVE_AGENT_DEMAND_PAGE_OK
AGENT_KERNEL_NATIVE_FAULT_HANDLER_AGENT_OK
AGENT_KERNEL_NATIVE_VERIFIER_OK
AGENT_KERNEL_NATIVE_RUNTIME_LOOP_OK
AGENT_KERNEL_NATIVE_RUNTIME_QUANTUM_OK
AGENT_KERNEL_AGENT_CALL_ABI_OK
AGENT_KERNEL_AGENT_CALL_RETURN_OK
AGENT_KERNEL_AGENT_CALL_AUTHORITY_OK
AGENT_KERNEL_AGENT_CALL_COMPLETE_OK
AGENT_KERNEL_AGENT_CR3_SWITCH_OK
AGENT_KERNEL_MULTI_AGENT_CONTEXT_SWITCH_OK
AGENT_KERNEL_HETEROGENEOUS_AGENT_EXECUTION_OK
AGENT_KERNEL_UART_IRQ_OK
AGENT_KERNEL_PORT_IO_BACKEND_OK
AGENT_KERNEL_PORT_COMMAND_FLOW_OK
AGENT_KERNEL_DRIVER_INVOCATION_FLOW_OK
event[1] agent_registered
event[2] capability_granted
event[3] agent_image_registered
event[4] agent_image_verified
event[5] agent_launched
event[6] observation
event[7] action
event[8] verification
event[9] driver_endpoint_registered
event[10] agent_registered
event[11] capability_derived
event[12] agent_image_registered
event[13] agent_image_verified
event[14] agent_launched
event[15] driver_bound
event[16] agent_registered
event[17] intent_declared
event[18] task_created
event[19] intent_bound
event[20] capability_derived
event[21] delegation
event[22] agent_registered
event[23] intent_declared
event[24] task_created
event[25] intent_bound
event[26] capability_derived
event[27] delegation
event[28] agent_image_registered
event[29] agent_image_verified
event[30] agent_image_registered
event[31] agent_image_verified
event[32] agent_launched
event[33] task_accepted
event[34] task_queued
event[35] agent_launched
event[36] task_accepted
event[37] task_queued
event[38] agent_registered
event[39] intent_declared
event[40] task_created
event[41] intent_bound
event[42] capability_derived
event[43] delegation
event[44] capability_derived
event[45] agent_image_registered
event[46] agent_image_verified
event[47] agent_launched
event[48] task_accepted
event[49] agent_registered
event[50] intent_declared
event[51] task_created
event[52] intent_bound
event[53] capability_derived
event[54] delegation
event[55] agent_image_registered
event[56] agent_image_verified
event[57] agent_launched
event[58] task_accepted
event[59] agent_registered
event[60] intent_declared
event[61] task_created
event[62] intent_bound
event[63] capability_derived
event[64] delegation
event[65] agent_image_registered
event[66] agent_image_verified
event[67] agent_launched
event[68] task_accepted
event[69] fault_handler_installed
event[70] fault_policy_installed
event[71] task_dispatched
event[72] task_quantum_expired
event[73] task_dispatched
event[74] task_quantum_expired
event[75] task_dispatched
event[76] message_wait_started
event[77] task_dispatched
event[78] task_result_submitted
event[79] message_sent
event[80] message_wait_woken
event[81] task_quantum_expired
event[82] task_dispatched
event[83] message_received
event[84] message_acknowledged
event[85] task_result_submitted
event[86] task_completed
event[87] task_dispatched
event[88] task_yielded
event[89] task_dispatched
event[90] task_completed
event[91] task_queued
event[92] task_dispatched
event[93] task_quantum_expired
event[94] task_dispatched
event[95] message_wait_started
event[96] task_queued
event[97] task_queued
event[98] task_dispatched
event[99] task_quantum_expired
event[100] task_dispatched
event[101] task_quantum_expired
event[102] task_dispatched
event[103] task_faulted
event[104] task_dispatched
event[105] task_result_inspected
event[106] task_verified
event[107] intent_fulfilled
event[108] task_completed
event[109] task_fault_recovered
event[110] task_queued
event[111] task_dispatched
event[112] task_quantum_expired
event[113] task_dispatched
event[114] task_faulted
event[115] task_fault_recovered
event[116] task_queued
event[117] task_dispatched
event[118] task_quantum_expired
event[119] task_dispatched
event[120] task_faulted
event[121] task_fault_recovered
event[122] task_queued
event[123] task_dispatched
event[124] task_quantum_expired
event[125] task_dispatched
event[126] task_faulted
event[127] message_sent
event[128] message_wait_woken
event[129] fault_routed
event[130] fault_policy_applied
event[131] task_dispatched
event[132] message_received
event[133] message_acknowledged
event[134] task_result_submitted
event[135] task_completed
event[136] task_fault_recovered
event[137] task_queued
event[138] task_dispatched
event[139] task_completed
event[140] device_event_raised
event[141] device_event_delivered
event[142] driver_invocation_queued
event[143] driver_invocation_dispatched
event[144] driver_invocation_ticked
event[145] device_event_acknowledged
event[146] driver_command_submitted
event[147] driver_command_dispatched
event[148] driver_command_completed
event[149] driver_invocation_completed
SUPERVISOR_HANDOFF_READY
```

`AGENT_KERNEL_GDT_TSS_OK` requires the permanent segment selectors and loaded
task register to match the host-tested GDT/TSS contract. The vector 3 trap gate
then captures the exact post-`int3` RIP and returns through `iretq` before
`AGENT_KERNEL_EXCEPTION_BASELINE_OK` is emitted.
`AGENT_KERNEL_AGENT_IMAGE_FORMAT_OK` requires three exact Worker Capsules, one
Verifier Capsule, and one FaultHandler Capsule with bounded canonical x86_64
V0 headers. `AGENT_KERNEL_AGENT_IMAGE_DIGEST_OK` requires all five computed
SHA-256 values, kinds, statuses, ABI versions, and entry versions to match their
distinct verified kernel records.
`AGENT_KERNEL_VERIFIER_IMAGE_OK` proves the same binding for Capsule kind 2 and
the verified Verifier record.
`AGENT_KERNEL_AGENT_USER_MEMORY_OK` requires successful fixed mappings for the
Agent RX code, read-only/NX signal, unmapped guard, and writable/NX stack pages,
plus one zeroed retained lazy frame whose initial leaf is absent.
`AGENT_KERNEL_AGENT_ADDRESS_SPACE_OK` additionally requires distinct aligned P4
roots, identical supervisor-only inherited mappings, an unused kernel P4 index
128, no kernel translation for any Agent-owned virtual page, and no initial
Agent translation for the lazy-data address.
`AGENT_KERNEL_VERIFIER_MEMORY_OK`, `AGENT_KERNEL_FAULT_HANDLER_MEMORY_OK`, and
`AGENT_KERNEL_MULTI_AGENT_MEMORY_OK` require five pairwise-disjoint
seven-content-frame memory identities behind the same virtual layout and a
common kernel root. `AGENT_KERNEL_AGENT_IMAGE_LOAD_OK` additionally requires
all five private code frames to match their verified payload byte-for-byte
before event 71 dispatches B.
`AGENT_KERNEL_PIT_IRQ_OK` requires the shared IDT's IRQ0 assembly entry to
capture each programmed PIT channel 0 quantum after hardware switched from
CPL3 to TSS RSP0. `AGENT_KERNEL_AGENT_CPU_PREEMPTION_OK` requires a complete
160-byte integer/privilege frame and a successful switch back to CPL0.
`AGENT_KERNEL_AGENT_RING3_PREEMPTION_OK` additionally requires exact user
selectors, in-range user RIP/RSP, IF set, IOPL clear, and an intact RSP0 canary.
`AGENT_KERNEL_AGENT_A_PREEMPTION_OK` additionally requires a second CR3 and a
second owned frame after RSP0 has been reused. `AGENT_KERNEL_TIMER_PREEMPTION_OK`
requires events 72 through 75 to preserve `[A, B]` then `[B, A]` FIFO rotation,
and event 81 to expire A's second quantum with cumulative run ticks 2 behind
the already-woken B.
`AGENT_KERNEL_KERNEL_SELECTED_DISPATCH_OK` additionally requires events 71, 73,
and 75 to return exact B, A, and B Agent/Task identities from the core without
an architecture-supplied Agent argument. The first two results must transfer
the matching prepared CPU objects while preserving the Verifier registration;
event 75 must recover B's matching preempted context.
The receive-wait markers require event 76 to bind B's still-owned call frame to
one active mailbox waiter, park it in the runtime registry, and use event 77 to
recover A's preempted context while B's task and execution context remain
`Waiting`. The blocking wake marker requires A's send to record
events 79 and 80 atomically, deactivate that waiter, and append B to the run
queue without returning from B's receive call early; event 82 must recover the
same waiting receive frame.
`AGENT_KERNEL_AGENT_CPU_RESUME_OK` requires both admission PIT frames, A's
session-preserving post-call PIT frame, and A's yielded call frame to resume at
their captured CPL3 RIP.
`AGENT_KERNEL_AGENT_CALL_RESULT_OK` requires A and B to
persist `{0x0a01, 0xa11c0001}` and `{0x0b02, 0xb22c0002}` in events 78 and 85
without leaving `Running`. `AGENT_KERNEL_AGENT_CALL_RETURNING_MUTATION_OK`
additionally requires each semantic result event and unchanged scheduler state
to be validated before its success reply can resume ring 3.

`AGENT_KERNEL_AGENT_CALL_SEND_MESSAGE_OK` requires operation 7 to create the
first bounded Notify from A to B at event 79 and return Message ID 1. The receive
and acknowledgement markers require B's operations 8 and 9 to validate that
record and move it exactly once through Received at event 83 to Acknowledged at
event 84. `AGENT_KERNEL_NATIVE_MAILBOX_IPC_OK` additionally requires the final
message payload to reference Task A, both results to remain stored, and all
three IPC mutations to preserve the expected scheduler state.
`AGENT_KERNEL_NATIVE_AGENT_YIELD_OK` requires A's operation 2 request to emit
event 88 after its post-Send quantum expiry and B's event 86 completion,
preserve its owned frame as Yielded, pass an Agent/Task/Yielded readiness check
at event 89, return a canonical Yield reply in ring 3, and complete A at event
90.

`AGENT_KERNEL_NATIVE_RUNTIME_STORE_OK` requires all five initial non-Copy CPU
contexts, all three consumed fault-to-prepared replacements, and the one
fault-to-repaired context to transfer only through kernel-returned Agent/Task
identities and leave the bounded registry empty after event 139.
`AGENT_KERNEL_VERIFIER_PREEMPTION_OK`
requires the Verifier's private CR3 to survive PIT expiry at event 101 and
redispatch after the contained fault at event 104. The inspection marker
requires operation 5 to echo the Verifier's scheduler-owned context and target
Worker A, emit event 105, return the stored result in R10/R11, and leave both
tasks and the run queue unchanged. `AGENT_KERNEL_AGENT_CALL_VERIFY_OK` requires
ring-3 code to compare those words before operation 6 can verify only A and emit
events 106 and 107. `AGENT_KERNEL_NATIVE_VERIFIER_OK` requires the fourth call to
complete the Verifier's own task at event 108, with three normal contexts Idle,
the Fault Worker context Faulted at that intermediate boundary, and Worker B
still Completed but unverified.
`AGENT_KERNEL_NATIVE_RUNTIME_LOOP_OK` additionally requires one role-independent
runtime loop to process all 21 calls plus four fault boundaries, cross prepared,
preempted, mailbox-waiting, yielded, and recovered-fault physical states, match
every terminal transcript to its Capsule nonce, operation sequence, and return
offsets, and finish with empty semantic and physical queues.
`AGENT_KERNEL_NATIVE_RUNTIME_QUANTUM_OK` additionally requires every ring-3
resume to start a fresh PIT quantum, the physical mailbox to classify each
return as exactly AgentCall, QuantumExpired, or AgentFault, A's generation to
advance from 1 to 2 while preserving its nonce and three-call transcript, event
82 to dispatch B before A, and all terminal generation counts to match their
Capsules.

`AGENT_KERNEL_NATIVE_FAULT_HANDLER_WAIT_OK` requires event 91 to queue Agent 7,
event 92 to dispatch its prepared context, event 93 to expire a real physical
quantum before its first call, and event 94 to recover that frame. Its
authenticated DescribeContext then precedes event 95, which binds the still-owned
ReceiveMessage frame to a live mailbox waiter.

`AGENT_KERNEL_NATIVE_AGENT_FAULT_CONTAINMENT_OK` requires event 98 to dispatch
the Fault Worker's prepared context, event 99 to give it physical generation 1,
and event 102 to resume the same owned frame. The vector-6 stub must distinguish
CPL3 from CPL0 using saved CS, capture `ud2` at Capsule offset 42 under the
Fault Worker's CR3, restore kernel CR3, and produce a valid
`AgentFault(InvalidOpcode)` mailbox. Public `sys_fault_task` must then record
event 103 as `ExecutionTrap` detail 6, leave no resumable Fault Worker context,
and allow event 104 to dispatch the Verifier. A kernel-origin #UD remains fatal.

`AGENT_KERNEL_NATIVE_AGENT_FAULT_RESTART_OK` requires the bootstrap capability
to carry explicit Rollback authority, event 109 to recover the same faulted task,
and event 110 to requeue it only after the semantic execution context becomes
Idle. The physical transition must consume `FaultedAgentCpu`, clear the signal
and all writable stack pages, set restart generation 1, and create a
`PreparedAgentCpu` at the immutable Capsule entry instead of resuming the saved
exception frame. Events 111 and 112 must prove a fresh entry and PIT admission;
event 113 must recover that new preemption frame before the generation-1 Capsule
executes `cli` at offset 44 and faults again at event 114.

`AGENT_KERNEL_NATIVE_AGENT_GENERAL_PROTECTION_OK` requires the vector-13 stub
to identify CPL3 from saved CS, read the CPU-pushed error code at frame offset
120, capture RIP at offset 128, restore kernel CR3, and classify exactly
`GeneralProtection { error_code: 0 }`. The semantic detail must be 13. Event 115
then performs a second authorized recovery, event 116 requeues only after the
execution context is Idle, and mutable pages are scrubbed before generation 2
is visible. Events 117 through 119 prove a second fresh entry and admission;
the generation-2 Capsule must then reach its read-only signal-page write.
Both earlier fault records remain queryable, and a kernel-origin #GP remains
fatal.

`AGENT_KERNEL_NATIVE_AGENT_PAGE_FAULT_OK` requires the vector-14 stub to check
saved CS, capture `CR2` before switching address spaces, read the CPU error code
at frame offset 120, and capture RIP at offset 128. The generation-2 write at
Capsule offset 47 must classify exactly `PageFault { error_code: 7, address:
0x0000400000001000 }` and event 120 must preserve semantic detail
`0xe007400000001000`. Event 121 performs the third authorized recovery, event
122 requeues only after the execution context is Idle, and all mutable pages
are scrubbed before generation 3 is visible. Events 123 through 125 prove a
fourth fresh entry and PIT admission. All three earlier fault records remain
queryable, and a kernel-origin #PF remains fatal.

`AGENT_KERNEL_NATIVE_AGENT_DEMAND_PAGE_OK` requires the generation-3 write at
Capsule offset 62 to fault at event 126 with exactly `PageFault { error_code: 6,
address: 0x0000400000007000 }` and semantic detail
`0xe006400000007000`. The retained frame must still contain zero and its leaf
must still be absent; no mapping changes before Handler approval.

`AGENT_KERNEL_NATIVE_FAULT_POLICY_ROUTE_OK` requires events 127 through 130 to
send Fault message 2, wake the exact Agent 7 waiter, record `FaultRouted`, and
record `FaultPolicyApplied` as one capacity-checked transition while task 4
remains Faulted. `AGENT_KERNEL_NATIVE_FAULT_HANDLER_DECISION_OK` requires event
131 to dispatch the retained receive frame and events 132 through 135 to
receive, acknowledge, submit `{0xf001, 4}`, and complete. The physical Capsule
must have nonce `0xe55ce005`, exactly five operations, return offsets
`45/54/122/143/152`, and ten Agent-call CR3 switches. Only that combined proof
can construct the FaultId-bound repair approval.

`AGENT_KERNEL_NATIVE_FAULT_HANDLER_AGENT_OK` requires the repair path to consume
that approval under bootstrap Rollback authority, map only the retained frame
user/writable/NX without allocation, and record recovery/queueing at events 136
and 137. Event 138 must consume a distinct `RecoveredFault` context and resume
the unchanged RIP. The retried write and readback produce `0x5a` before return
offsets 105 and 114 complete task 4 at event 139. All four fault records remain
queryable, restart generation stays 3, and the final task retains four run
ticks.

`AGENT_KERNEL_RESUMABLE_RUNTIME_REGISTRY_OK` additionally requires every
demonstrated non-running prepared, PIT-preempted, mailbox-waiting, yielded, or
`RecoveredFault` CPU context to be parked under its trusted Agent identity,
selected by the exact kernel-returned Agent/Task pair, and consumed by the
terminal boundary.
`AGENT_KERNEL_DISPATCH_READINESS_HANDOFF_OK` additionally requires all twenty-one
Worker, Fault Worker, Verifier, and Fault Handler handoffs to use the same mutable dispatch-and-take
operation. That operation prepares a read-only core permit, finds a parked CPU
state from the permit's Agent/Task identity, commits the semantic transition,
and consumes the guarded context. No caller supplies an expected Agent or
physical kind. Permit preparation emits no event, so the ordered trace remains
exactly 149 events.

`AGENT_KERNEL_AGENT_CALL_ABI_OK` requires five canonical `AGNTCALL` requests
from sender A, five from receiver B, four from the Verifier, two from the
restarted Fault Worker, and five from the Fault Handler with version 1,
supported operations, zero flags,
operation-specific registers, and zero reserved words. The return marker
requires context, task-result, mailbox,
Yield, inspection, and verification replies to return trusted identity, nonce,
and operation-specific payloads through owned `iretq` frames before each
terminal CompleteTask request.
`AGENT_KERNEL_AGENT_CALL_AUTHORITY_OK` requires each physical request to match
the scheduler context and kernel-held capability appropriate to that operation;
the Verifier's resource Verify capability never appears in ring-3 registers.
`AGENT_KERNEL_AGENT_CALL_COMPLETE_OK` requires core authorization to accept
those capabilities, produce Worker completion events 86 and 90, Verifier
completion event 108, Handler completion event 135, and repaired Fault Worker
completion event 139, leaving all five task execution contexts Idle with an
empty run queue.
`AGENT_KERNEL_AGENT_CR3_SWITCH_OK` requires every PIT, Agent-call, #UD, #GP, and #PF
entry to observe its private Agent CR3 and every return to normal context to
restore the kernel CR3.
`AGENT_KERNEL_MULTI_AGENT_ISOLATION_OK` requires B's retained receive frame,
the read-only signal pages, the initially absent private lazy frames, and A's
preempted then yielded frames to remain intact across the opposite Worker's
execution.
`AGENT_KERNEL_MULTI_AGENT_CONTEXT_SWITCH_OK` requires ten Agent-call CR3
transitions for A, ten for B, eight for the Verifier, four for the restarted
Fault Worker, and ten for the Fault Handler. Nine PIT round trips include A's
live call session, the Verifier, the Handler's pre-call admission, and all four
Fault Worker generations, followed by Fault Worker #UD, #GP, and two #PF
returns, with all terminal scheduler states.
`AGENT_KERNEL_HETEROGENEOUS_AGENT_EXECUTION_OK`
requires the two different Worker call sequences, the Verifier sequence, the
restarted Fault Worker sequence, and the Fault Handler decision sequence to
return at their verified offsets with distinct nonces.
`AGENT_KERNEL_UART_IRQ_OK` requires IRQ4 to capture one THRE interrupt and normal
context to validate its IIR/LSR mailbox. That signal causes event 140 and the
running Driver Invocation. The `O` in `AGENT_KERNEL_PORT_IO_BACKEND_OK` is
emitted by the immutable causal request at event 146 through the registered COM1
endpoint and `PortIoBackend`; the surrounding text uses the existing boot
serial writer. The remaining success markers are printed only after the write
and Driver Invocation terminal records are verified as `Completed` and the
Driver execution context is `Idle`.

The x86 entry configures a 1 MiB kernel stack plus a separate 32 KiB TSS RSP0
stack. The fixed-capacity kernel state
and its by-value boot construction exceed the bootloader's default 80 KiB stack
in unoptimized QEMU builds; the explicit size keeps the guard page effective
while making the debug boot contract deterministic.

`scripts/run-qemu.sh` treats QEMU exit code `33` as success because the kernel
exits through the `isa-debug-exit` device with value `0x10`.

Expected supervisor output:

```text
Agent Kernel supervisor boot
event[1] agent_registered agent=1 target_agent=1
event[2] agent_registered agent=2 target_agent=2
event[3] agent_registered agent=3 target_agent=3
event[4] capability_granted agent=1 resource=1 capability=1
event[5] agent_image_registered agent=1 resource=1 capability=1 image=1 kind=supervisor
event[6] agent_image_verified agent=1 resource=1 capability=1 image=1 kind=supervisor
event[7] agent_launched agent=1 resource=1 capability=1 image=1
event[8] fault_handler_installed agent=1 resource=1 target_agent=3
event[9] fault_policy_installed agent=1 resource=1 policy=1 action=route_to_handler
event[10] observation agent=1 resource=1
event[11] action agent=1 resource=1 action=1
event[12] verification agent=1 resource=1 action=1
event[13] checkpoint agent=1 resource=1 checkpoint=1
event[14] rollback agent=1 resource=1 checkpoint=1
event[15] intent_declared agent=1 resource=1 intent=1
event[16] task_created agent=1 resource=1 task=1
event[17] intent_bound agent=1 resource=1 intent=1
event[18] capability_derived agent=1 resource=1 capability=2
event[19] delegation agent=1 resource=1 task=1 target_agent=2
event[20] agent_image_registered agent=1 resource=1 capability=1 image=2 kind=worker
event[21] agent_image_verified agent=1 resource=1 capability=1 image=2 kind=worker
event[22] agent_launched agent=2 resource=1 capability=2 image=2 task=1
event[23] task_accepted agent=2 resource=1 task=1
event[24] task_queued agent=2 resource=1 task=1
event[25] task_dispatched agent=2 resource=1 task=1
event[26] task_ticked agent=2 resource=1 task=1 ticks=1 quantum=1
event[27] task_quantum_expired agent=2 resource=1 task=1 ticks=2 quantum=0
event[28] task_dispatched agent=2 resource=1 task=1
event[29] task_faulted agent=2 resource=1 task=1 fault=1 detail=7
event[30] message_sent agent=1 target_agent=3 message=1
event[31] fault_routed agent=1 resource=1 task=1 fault=1 detail=7 target_agent=3 message=1
event[32] fault_policy_applied agent=1 resource=1 task=1 fault=1 detail=7 policy=1 action=route_to_handler message=1
event[33] message_received agent=3 target_agent=1 message=1
event[34] message_acknowledged agent=3 target_agent=1 message=1
event[35] task_fault_recovered agent=1 resource=1 task=1 fault=1 detail=7
event[36] task_queued agent=2 resource=1 task=1
event[37] task_dispatched agent=2 resource=1 task=1
event[38] task_waiting agent=2 resource=1 task=1 waiter=1 signal=1
event[39] signal_emitted agent=1 resource=1 task=1 waiter=1 signal=1 target_agent=2
event[40] task_woken agent=1 resource=1 task=1 waiter=1 signal=1 target_agent=2
event[41] task_dispatched agent=2 resource=1 task=1
event[42] task_completed agent=2 resource=1 task=1
event[43] task_verified agent=1 resource=1 task=1
event[44] intent_fulfilled agent=1 resource=1 intent=1
event[45] message_sent agent=1 target_agent=2 message=2
event[46] message_received agent=2 target_agent=1 message=2
event[47] message_acknowledged agent=2 target_agent=1 message=2
event[48] capability_granted agent=1 resource=2 capability=3
event[49] memory_cell_created agent=1 resource=2 memory_cell=1
event[50] memory_cell_recalled agent=1 resource=2 memory_cell=1
event[51] memory_cell_remembered agent=1 resource=2 memory_cell=1
event[52] namespace_entry_bound agent=1 resource=1 namespace_entry=1 key=1
event[53] namespace_entry_resolved agent=1 resource=1 namespace_entry=1 key=1
event[54] namespace_entry_rebound agent=1 resource=1 namespace_entry=1 key=1
event[55] resource_created agent=1 resource=3 capability=4
event[56] capability_granted agent=1 resource=3 capability=4
event[57] resource_retired agent=1 resource=3 capability=4
event[58] capability_derived agent=1 resource=1 capability=5
event[59] observation agent=2 resource=1
event[60] agent_registered agent=4 target_agent=4
event[61] resource_created agent=1 resource=4 capability=6
event[62] capability_granted agent=1 resource=4 capability=6
event[63] driver_endpoint_registered agent=1 resource=4 capability=6
event[64] capability_derived agent=1 resource=4 capability=7
event[65] agent_image_registered agent=1 resource=4 capability=6 image=3 kind=driver
event[66] agent_image_verified agent=1 resource=4 capability=6 image=3 kind=driver
event[67] agent_launched agent=4 resource=4 capability=7 image=3
event[68] driver_bound agent=1 resource=4 capability=6 driver_binding=1 target_agent=4
event[69] device_event_raised agent=1 resource=4 capability=6 driver_binding=1 device_event=1 driver_invocation=0 kind=state_changed code=1 value=2
event[70] device_event_delivered agent=4 resource=4 capability=7 driver_binding=1 device_event=1 driver_invocation=1 kind=state_changed code=1 value=2
event[71] driver_invocation_queued agent=4 resource=4 capability=7 driver_binding=1 device_event=1 driver_invocation=1 ticks=0 quantum=0
event[72] driver_invocation_dispatched agent=4 resource=4 capability=0 driver_binding=1 device_event=1 driver_invocation=1 ticks=0 quantum=2
event[73] driver_invocation_ticked agent=4 resource=4 capability=0 driver_binding=1 device_event=1 driver_invocation=1 ticks=1 quantum=1
event[74] device_event_acknowledged agent=4 resource=4 capability=7 driver_binding=1 device_event=1 driver_invocation=1 kind=state_changed code=1 value=2
event[75] driver_command_submitted agent=4 resource=4 capability=7 driver_binding=1 device_event=1 driver_invocation=1 driver_command=1 kind=write opcode=3 value=11
event[76] driver_command_dispatched agent=4 resource=4 capability=7 driver_binding=1 device_event=1 driver_invocation=1 driver_command=1 kind=write opcode=3 value=11
event[77] driver_command_completed agent=4 resource=4 capability=7 driver_binding=1 device_event=1 driver_invocation=1 driver_command=1 kind=write opcode=3 value=11 result_code=0 result_value=11
event[78] driver_invocation_completed agent=4 resource=4 capability=7 driver_binding=1 device_event=1 driver_invocation=1 ticks=1 quantum=0
```
