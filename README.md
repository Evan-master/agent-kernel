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
- `agent-kernel-x86_64`: no_std x86_64 bootloader entry, native one-page Worker and Verifier Agent Image Capsule parsing with SHA-256 verification binding, four isolated Agent CR3 roots with same-address private pages, a fixed-capacity prepared/preempted/mailbox-waiting/yielded CPU ownership registry consumed by a kernel-selected dispatch-and-take runtime loop, owned suspended CPU frames, PIT IRQ0 quanta re-armed before every ring-3 entry or resume through a shared RSP0 stack, strict AgentCall/QuantumExpired/AgentFault physical boundary classification, CPL-aware #UD and error-code-bearing #GP containment into kernel `ExecutionTrap` state, two consuming fault-to-fresh-entry restarts with signal/stack scrubbing, a read-only per-Agent call-release/quantum/restart signal page, a versioned returning Agent Call ABI whose role-independent inner loop routes all nine v1 operations and records fixed-capacity call transcripts, cooperative Yield, blocking mailbox wait/wake plus Send/Receive/Acknowledge between isolated Workers, audited task-result submission and inspection, target-scoped verification, task completion, one-shot UART IRQ4 ingress, and byte-wide Port I/O behind the privileged Driver boundary.
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
validates the exact CPU return RIP. Vectors 6 and 13 are later replaced by
CPL-aware Agent gates: kernel-origin #UD/#GP still reach deterministic fatal
stubs, while ring-3 #UD captures a 160-byte frame and ring-3 #GP captures a
168-byte frame plus its CPU error code before returning to trusted kernel
context. All remaining exception gates lead to vector-specific deterministic
failure stubs. Before that IDT becomes active,
the kernel installs a permanent GDT with ring-0/ring-3 segments and a long-mode
TSS whose RSP0 points at a dedicated 32 KiB privileged stack. The boot adapter
then registers and verifies two normal Worker images, one Fault Worker image,
and one Verifier image.
Each immutable native Capsule has a fixed 32-byte AgentOS header followed by at
most one code page; the no_std loader rejects unsupported format, architecture,
kind, or flags, zero or mismatched ABI and entry versions, noncanonical lengths,
reserved data, and out-of-range entry offsets. It computes SHA-256 across the
exact header and code bytes and requires equality with the verified kernel image
record before any task can dispatch. A bounded allocator then consumes only
BootInfo `Usable` frames and creates four distinct Agent P4 roots. All four
inherit identical supervisor-only kernel mappings, while each dedicated P4
index 128 contains private frames at the same virtual addresses: one read-only
executable Agent code page, one read-only/NX signal page, an unmapped guard page,
and four writable/NX stack pages. The kernel CR3 has no translation for any
Agent region, and all root/code/signal/stack physical frames are pairwise
disjoint.

Each code page is zeroed, filled only from its verified Capsule, and read back
through the supervisor physical alias before mapping. Verifier registration,
task delegation, its separate resource-scoped Verify capability, image
verification, launch, and acceptance occupy events 38 through 48. Fault Worker
registration, task delegation, image verification, launch, and acceptance
occupy events 49 through 58. The four prepared non-Copy CPU objects first enter
a bounded native runtime registry.
The role-independent outer loop asks the core for its FIFO-head permit, finds a
matching owned CPU state using only the returned Agent/Task identity, commits
the dispatch, and consumes that exact state. Before event 59 this selects Worker
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
releases RSP0 for the next Agent. Events 60 through 63 expire B and A in turn,
park each complete preemption frame, and let successive outer-loop iterations
select A's prepared then B's preempted CPU while preserving queue order.
Every later return to ring 3 also starts a fresh PIT quantum. A reset physical
mailbox must classify as exactly one Agent Call, one timer expiry, or one
supported Agent fault; mixed or repeated evidence fails closed. A validated
expiry increments byte 1 of that Agent's read-only signal page without
replacing the public semantic task tick. Byte 2 is reserved for a
kernel-authored restart generation, starts at zero, and is bounded at two.
Nonce, transcript, private memory, and the complete frame remain owned when a
live call session becomes Preempted.

The kernel first releases B's read-only signal through its supervisor alias and
resumes B through DescribeContext and ReceiveMessage. Because its mailbox is
empty, event 64 atomically moves B's task and execution context to `Waiting` and
binds a mailbox waiter to the captured receive frame. The native registry parks
that waiting frame before A's preempted context passes event 65's permit
preflight, dispatch commit, and guarded recovery. Worker A performs five
returning calls:
DescribeContext, SubmitTaskResult, SendMessage, Yield, and CompleteTask. It
sends a Notify carrying its Task ID to Worker B and compares the returned
deterministic Message ID before yielding. Only an exact scheduler-owned
Agent/Task/Image identity and nonce echo can mutate task or mailbox state. A's result is event
66; its send records the message at event 67 and atomically wakes and requeues B
at event 68. After the Send reply, A waits for its read-only physical quantum
generation to advance from 1 to 2. The re-armed PIT captures that live session,
event 69 expires A's second semantic quantum, and the FIFO becomes `[B, A]`
without adding a transcript entry. B redispatches at event 70 only after its
waiting frame matches the prepared permit. The kernel receives Message ID 1 at event 71 before encoding
the reply into B's original saved frame. B then
calls AcknowledgeMessage, SubmitTaskResult, and CompleteTask
at events 72 through 74. Its ring-3 code validates Message ID 1, sender Agent 3,
Notify kind, and Task ID 1; the kernel independently validates the same record.
A redispatches at event 75 only after the same session-preserving preempted
Agent/Task frame passes readiness. It exits the generation wait, validates the
Send reply, yields at event 76, resumes that acknowledged Yield frame at event
77, validates the Yield reply, and completes at event 78. A uses a 148-byte
image with return offsets 46/67/94/131/146. B
uses a 131-byte image with a two-NOP prefix, a different nonce, and return
offsets 48/57/99/120/129. The terminal mailbox record is Acknowledged while both
fixed-width task results remain stored.

The kernel then queues the Fault Worker and Verifier at events 79 and 80. The
Fault Worker dispatches first, expires its admission quantum at event 82, and
rotates behind the Verifier. The Verifier dispatches and expires at events 83
and 84, restoring the FIFO to `[Fault Worker, Verifier]`. Event 85 resumes the
Fault Worker's exact saved frame. Its 81-byte code observes physical quantum
generation 1 and, while restart generation is zero, executes `ud2` at code
offset 36. The vector-6 gate verifies CPL3 origin, saves every integer register
and the privilege frame, switches back to kernel CR3, and classifies the return
as `AgentFault(InvalidOpcode)`. Event 86 records `ExecutionTrap` detail 6 and
leaves only that task and execution context `Faulted`.

The Verifier redispatches after the fault at event 87, proving that one Agent's
invalid instruction did not terminate the kernel or another Agent. Its first
returning call describes its trusted context. Its second call inspects only
Worker A's stored result under resource-scoped Verify authority and emits the
audited `TaskResultInspected` event 88 without mutating scheduler state.
Ring-3 machine code compares the returned words with `0x0a01` and
`0xa11c0001`; a mismatch enters a terminal loop. Reaching its third call proves
that the comparison succeeded before Worker A becomes Verified at event 89 and
its intent becomes Fulfilled at event 90. The fourth call completes the
Verifier's own task at event 91. Worker B remains Completed with its different
result and bound intent as a target-scoping control. The Verifier uses a
111-byte image with DescribeContext/InspectTaskResult/VerifyTask/CompleteTask
return offsets 46/64/100/109. At this intermediate boundary the three normal
execution contexts are Idle, the Fault Worker remains Faulted, and both queues
are empty.

The physical fault report then consumes the Fault Worker's non-resumable
exception object. It clears the complete private signal page and all four
writable stack pages, writes restart generation 1 through the supervisor alias,
and passes the retained verified code mapping and CR3 root back through the
prepared-entry validator. The saved exception frame is dropped and cannot be
used as a resume source. Event 92 records bootstrap-authorized
`TaskFaultRecovered`; event 93 requeues the recovered task under its assigned
Agent. A fresh entry dispatch at event 94 expires through PIT at event 95, and
event 96 resumes that new admission frame. The Capsule observes restart
generation 1 and executes privileged `cli` at code offset 38. The vector-13
gate validates CPL3 origin, captures its 168-byte frame and CPU error code zero,
restores kernel CR3, and classifies `GeneralProtection { error_code: 0 }`.
Event 97 records a second immutable `ExecutionTrap`, now with detail 13.

That second non-resumable fault object is consumed through the same scrubbed
fresh-entry transition, which writes restart generation 2. Event 98 performs a
second bootstrap-authorized recovery and event 99 requeues the task. Events 100
and 101 prove another fresh entry and PIT admission; event 102 resumes only the
new preemption frame. At generation 2 the Capsule authenticates DescribeContext
with nonce `0xd44ce004` and completes through the ordinary Agent Call path at
event 103. Both fault records remain attached to the audit history while the
task becomes Completed.
All four Agent execution contexts finish Idle, and semantic and physical queues
are empty. This is an Agent-native image format, call ABI, scheduler, fault and
restart lifecycle, and verification lifecycle, not a POSIX process or syscall
ABI.
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
unbounded restart policy, replacement-page allocation, checkpoint data
restoration, containment of page faults or exceptions other than #UD/#GP,
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
4. Allocate four distinct Agent P4 roots, inherit supervisor kernel mappings, and map disjoint fixed CPL3 code, signal, guard, and stack regions at identical virtual addresses.
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
19. Parse all four fixed Capsule headers, bind their SHA-256 digests to the verified records, copy/read back all four private code pages, and register their prepared CPU ownership by trusted Agent identity.
20. Replace IDT vectors 6 and 13 with CPL-aware Agent #UD/#GP gates while retaining both original CPL0 fatal fallbacks.
21. Let the kernel-selected outer loop dispatch B and A from their prepared states, expire each admission quantum through IRQ0, and park both validated preemption frames.
22. Redispatch B from its preempted state and enter the generic inner loop through DescribeContext.
23. Route B's empty ReceiveMessage into a retained waiting call, then let the outer loop select A's preempted state while B remains `Waiting`.
24. Route A's result submission and typed Notify send, atomically wake B, then re-arm PIT before A observes its read-only quantum generation and capture the live call session at generation 2.
25. Expire A's second semantic quantum behind B, resume B's retained ReceiveMessage, and route B's acknowledgement, result, and completion.
26. Redispatch A's session-preserving preempted frame, route its Yield, redispatch the acknowledged Yield frame, complete A, and match both Worker transcripts and quantum generations.
27. Queue Fault Worker before Verifier, expire both admission quanta, and restore FIFO order with Fault Worker first.
28. Resume Fault Worker, capture its exact `ud2` frame as `AgentFault(InvalidOpcode)`, and commit one `ExecutionTrap` without parking a resumable context.
29. Redispatch Verifier after the fault and route DescribeContext, InspectTaskResult, VerifyTask, and CompleteTask while B remains unverified.
30. Consume the #UD object, scrub its signal and stack pages, set restart generation 1, recover through bootstrap rollback authority, and requeue a fresh prepared context.
31. Expire that fresh admission quantum, execute privileged `cli`, decode the vector-13 CPU error code, and commit a second immutable `ExecutionTrap` without retaining a resumable context.
32. Consume the #GP object, scrub mutable pages again, set restart generation 2, perform a second authorized recovery, and requeue another fresh prepared context.
33. Expire that admission quantum and complete through authenticated DescribeContext/CompleteTask calls while retaining both fault records.
34. Match all terminal evidence, prove fifteen kernel-selected dispatches, seven physical quantum expiries, two contained Agent faults, four completed Idle contexts, no faulted physical context, and empty queues.
35. Install IRQ4 in the persistent IDT, remap the PIC, arm COM1 THRE, and receive the hardware interrupt.
36. Validate the interrupt mailbox, raise and deliver an Interrupt Device Event, then dispatch and tick its Driver Invocation.
37. Acknowledge the event and dispatch a causally linked COM1 write request.
38. Record the write result, complete the Driver Invocation, and mark supervisor handoff ready.

The handoff now runs inside QEMU through the x86_64 BIOS image path.

## Non-Goals For V0

- Booting on physical hardware.
- UEFI image support.
- POSIX compatibility.
- Linux syscall compatibility.
- A filesystem, network stack, dynamic/SMP multi-Agent scheduler, or complete physical hardware driver stack.
- Dynamic per-task PIT frequencies or multi-tick hardware quantum lengths.
- Automatic retry/crash-loop policy, more than two fault restarts, replacement address spaces, or containment of CPU exceptions other than ring-3 #UD/#GP.
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
AGENT_KERNEL_NATIVE_RUNTIME_STORE_OK
AGENT_KERNEL_VERIFIER_PREEMPTION_OK
AGENT_KERNEL_AGENT_CALL_INSPECT_RESULT_OK
AGENT_KERNEL_AGENT_CALL_VERIFY_OK
AGENT_KERNEL_RESUMABLE_RUNTIME_REGISTRY_OK
AGENT_KERNEL_DISPATCH_READINESS_HANDOFF_OK
AGENT_KERNEL_NATIVE_AGENT_FAULT_CONTAINMENT_OK
AGENT_KERNEL_NATIVE_AGENT_FAULT_RESTART_OK
AGENT_KERNEL_NATIVE_AGENT_GENERAL_PROTECTION_OK
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
event[59] task_dispatched
event[60] task_quantum_expired
event[61] task_dispatched
event[62] task_quantum_expired
event[63] task_dispatched
event[64] message_wait_started
event[65] task_dispatched
event[66] task_result_submitted
event[67] message_sent
event[68] message_wait_woken
event[69] task_quantum_expired
event[70] task_dispatched
event[71] message_received
event[72] message_acknowledged
event[73] task_result_submitted
event[74] task_completed
event[75] task_dispatched
event[76] task_yielded
event[77] task_dispatched
event[78] task_completed
event[79] task_queued
event[80] task_queued
event[81] task_dispatched
event[82] task_quantum_expired
event[83] task_dispatched
event[84] task_quantum_expired
event[85] task_dispatched
event[86] task_faulted
event[87] task_dispatched
event[88] task_result_inspected
event[89] task_verified
event[90] intent_fulfilled
event[91] task_completed
event[92] task_fault_recovered
event[93] task_queued
event[94] task_dispatched
event[95] task_quantum_expired
event[96] task_dispatched
event[97] task_faulted
event[98] task_fault_recovered
event[99] task_queued
event[100] task_dispatched
event[101] task_quantum_expired
event[102] task_dispatched
event[103] task_completed
event[104] device_event_raised
event[105] device_event_delivered
event[106] driver_invocation_queued
event[107] driver_invocation_dispatched
event[108] driver_invocation_ticked
event[109] device_event_acknowledged
event[110] driver_command_submitted
event[111] driver_command_dispatched
event[112] driver_command_completed
event[113] driver_invocation_completed
SUPERVISOR_HANDOFF_READY
```

`AGENT_KERNEL_GDT_TSS_OK` requires the permanent segment selectors and loaded
task register to match the host-tested GDT/TSS contract. The vector 3 trap gate
then captures the exact post-`int3` RIP and returns through `iretq` before
`AGENT_KERNEL_EXCEPTION_BASELINE_OK` is emitted.
`AGENT_KERNEL_AGENT_IMAGE_FORMAT_OK` requires three exact Worker Capsules and
one Verifier Capsule with bounded canonical x86_64 V0 headers.
`AGENT_KERNEL_AGENT_IMAGE_DIGEST_OK` requires both normal Workers and the Fault
Worker's computed SHA-256 values, kind, status, ABI version, and entry version
to match their distinct verified kernel records.
`AGENT_KERNEL_VERIFIER_IMAGE_OK` proves the same binding for Capsule kind 2 and
the verified Verifier record.
`AGENT_KERNEL_AGENT_USER_MEMORY_OK` requires successful fixed mappings for the
Agent RX code, read-only/NX signal, unmapped guard, and writable/NX stack pages.
`AGENT_KERNEL_AGENT_ADDRESS_SPACE_OK` additionally requires distinct aligned P4
roots, identical supervisor-only inherited mappings, an unused kernel P4 index
128, and no kernel translation for any Agent-owned virtual page.
`AGENT_KERNEL_VERIFIER_MEMORY_OK` and `AGENT_KERNEL_MULTI_AGENT_MEMORY_OK`
require four pairwise-disjoint physical memory identities behind the same
virtual layout and a common kernel root. `AGENT_KERNEL_AGENT_IMAGE_LOAD_OK`
additionally requires all four private code frames to match their verified
payload byte-for-byte before event 59 dispatches B.
`AGENT_KERNEL_PIT_IRQ_OK` requires the shared IDT's IRQ0 assembly entry to
capture each programmed PIT channel 0 quantum after hardware switched from
CPL3 to TSS RSP0. `AGENT_KERNEL_AGENT_CPU_PREEMPTION_OK` requires a complete
160-byte integer/privilege frame and a successful switch back to CPL0.
`AGENT_KERNEL_AGENT_RING3_PREEMPTION_OK` additionally requires exact user
selectors, in-range user RIP/RSP, IF set, IOPL clear, and an intact RSP0 canary.
`AGENT_KERNEL_AGENT_A_PREEMPTION_OK` additionally requires a second CR3 and a
second owned frame after RSP0 has been reused. `AGENT_KERNEL_TIMER_PREEMPTION_OK`
requires events 60 through 63 to preserve `[A, B]` then `[B, A]` FIFO rotation,
and event 69 to expire A's second quantum with cumulative run ticks 2 behind
the already-woken B.
`AGENT_KERNEL_KERNEL_SELECTED_DISPATCH_OK` additionally requires events 59, 61,
and 63 to return exact B, A, and B Agent/Task identities from the core without
an architecture-supplied Agent argument. The first two results must transfer
the matching prepared CPU objects while preserving the Verifier registration;
event 63 must recover B's matching preempted context.
The receive-wait markers require event 64 to bind B's still-owned call frame to
one active mailbox waiter, park it in the runtime registry, and use event 65 to
recover A's preempted context while B's task and execution context remain
`Waiting`. The blocking wake marker requires A's send to record
events 67 and 68 atomically, deactivate that waiter, and append B to the run
queue without returning from B's receive call early; event 70 must recover the
same waiting receive frame.
`AGENT_KERNEL_AGENT_CPU_RESUME_OK` requires both admission PIT frames, A's
session-preserving post-call PIT frame, and A's yielded call frame to resume at
their captured CPL3 RIP.
`AGENT_KERNEL_AGENT_CALL_RESULT_OK` requires A and B to
persist `{0x0a01, 0xa11c0001}` and `{0x0b02, 0xb22c0002}` in events 66 and 73
without leaving `Running`. `AGENT_KERNEL_AGENT_CALL_RETURNING_MUTATION_OK`
additionally requires each semantic result event and unchanged scheduler state
to be validated before its success reply can resume ring 3.

`AGENT_KERNEL_AGENT_CALL_SEND_MESSAGE_OK` requires operation 7 to create the
first bounded Notify from A to B at event 67 and return Message ID 1. The receive
and acknowledgement markers require B's operations 8 and 9 to validate that
record and move it exactly once through Received at event 71 to Acknowledged at
event 72. `AGENT_KERNEL_NATIVE_MAILBOX_IPC_OK` additionally requires the final
message payload to reference Task A, both results to remain stored, and all
three IPC mutations to preserve the expected scheduler state.
`AGENT_KERNEL_NATIVE_AGENT_YIELD_OK` requires A's operation 2 request to emit
event 76 after its post-Send quantum expiry and B's event 74 completion,
preserve its owned frame as Yielded, pass an Agent/Task/Yielded readiness check
at event 77, return a canonical Yield reply in ring 3, and complete A at event
78.

`AGENT_KERNEL_NATIVE_RUNTIME_STORE_OK` requires all four initial non-Copy CPU
contexts plus both consumed fault-to-prepared replacements to transfer only
through kernel-returned Agent/Task identities and leave the bounded registry
empty after event 103. `AGENT_KERNEL_VERIFIER_PREEMPTION_OK`
requires the Verifier's private CR3 to survive PIT expiry at event 84 and
redispatch after the contained fault at event 87. The inspection marker
requires operation 5 to echo the Verifier's scheduler-owned context and target
Worker A, emit event 88, return the stored result in R10/R11, and leave both
tasks and the run queue unchanged. `AGENT_KERNEL_AGENT_CALL_VERIFY_OK` requires
ring-3 code to compare those words before operation 6 can verify only A and emit
events 89 and 90. `AGENT_KERNEL_NATIVE_VERIFIER_OK` requires the fourth call to
complete the Verifier's own task at event 91, with three normal contexts Idle,
the Fault Worker context Faulted at that intermediate boundary, and Worker B
still Completed but unverified.
`AGENT_KERNEL_NATIVE_RUNTIME_LOOP_OK` additionally requires one role-independent
runtime loop to process all 16 calls plus two fault boundaries, cross prepared,
preempted, mailbox-waiting, and yielded physical states, match every terminal
transcript to its Capsule nonce, operation sequence, and return offsets, and
finish with empty semantic and physical queues.
`AGENT_KERNEL_NATIVE_RUNTIME_QUANTUM_OK` additionally requires every ring-3
resume to start a fresh PIT quantum, the physical mailbox to classify each
return as exactly AgentCall, QuantumExpired, or AgentFault, A's generation to
advance from 1 to 2 while preserving its nonce and three-call transcript, event
70 to dispatch B before A, and all terminal generation counts to match their
Capsules.

`AGENT_KERNEL_NATIVE_AGENT_FAULT_CONTAINMENT_OK` requires event 81 to dispatch
the Fault Worker's prepared context, event 82 to give it physical generation 1,
and event 85 to resume the same owned frame. The vector-6 stub must distinguish
CPL3 from CPL0 using saved CS, capture `ud2` at Capsule offset 36 under the
Fault Worker's CR3, restore kernel CR3, and produce a valid
`AgentFault(InvalidOpcode)` mailbox. Public `sys_fault_task` must then record
event 86 as `ExecutionTrap` detail 6, leave no resumable Fault Worker context,
and allow event 87 to dispatch the Verifier. A kernel-origin #UD remains fatal.

`AGENT_KERNEL_NATIVE_AGENT_FAULT_RESTART_OK` requires the bootstrap capability
to carry explicit Rollback authority, event 92 to recover the same faulted task,
and event 93 to requeue it only after the semantic execution context becomes
Idle. The physical transition must consume `FaultedAgentCpu`, clear the signal
and all writable stack pages, set restart generation 1, and create a
`PreparedAgentCpu` at the immutable Capsule entry instead of resuming the saved
exception frame. Events 94 and 95 must prove a fresh entry and PIT admission;
event 96 must recover that new preemption frame before the generation-1 Capsule
executes `cli` at offset 38 and faults again at event 97.

`AGENT_KERNEL_NATIVE_AGENT_GENERAL_PROTECTION_OK` requires the vector-13 stub
to identify CPL3 from saved CS, read the CPU-pushed error code at frame offset
120, capture RIP at offset 128, restore kernel CR3, and classify exactly
`GeneralProtection { error_code: 0 }`. The semantic detail must be 13. Event 98
then performs a second authorized recovery, event 99 requeues only after the
execution context is Idle, and mutable pages are scrubbed before generation 2
is visible. Events 100 through 102 prove a second fresh entry and admission;
authenticated DescribeContext/CompleteTask completes at event 103. Both fault
records remain queryable, and a kernel-origin #GP remains fatal.

`AGENT_KERNEL_RESUMABLE_RUNTIME_REGISTRY_OK` additionally requires every
demonstrated non-running prepared, PIT-preempted, mailbox-waiting, or yielded CPU
context to be parked under its trusted Agent identity, selected by the exact
kernel-returned Agent/Task pair, and consumed by the terminal boundary.
`AGENT_KERNEL_DISPATCH_READINESS_HANDOFF_OK` additionally requires all fifteen
Worker, Fault Worker, and Verifier handoffs to use the same mutable dispatch-and-take
operation. That operation prepares a read-only core permit, finds a parked CPU
state from the permit's Agent/Task identity, commits the semantic transition,
and consumes the guarded context. No caller supplies an expected Agent or
physical kind. Permit preparation emits no event, so the ordered trace remains
exactly 113 events.

`AGENT_KERNEL_AGENT_CALL_ABI_OK` requires five canonical `AGNTCALL` requests
from sender A, five from receiver B, four from the Verifier, and two from the
restarted Fault Worker with version 1, supported operations, zero flags,
operation-specific registers, and zero reserved words. The return marker
requires context, task-result, mailbox,
Yield, inspection, and verification replies to return trusted identity, nonce,
and operation-specific payloads through owned `iretq` frames before each
terminal CompleteTask request.
`AGENT_KERNEL_AGENT_CALL_AUTHORITY_OK` requires each physical request to match
the scheduler context and kernel-held capability appropriate to that operation;
the Verifier's resource Verify capability never appears in ring-3 registers.
`AGENT_KERNEL_AGENT_CALL_COMPLETE_OK` requires core authorization to accept
those capabilities, produce Worker completion events 74 and 78, Verifier
completion event 91, and twice-recovered Fault Worker completion event 103, leaving
all four task execution contexts Idle with an empty run queue.
`AGENT_KERNEL_AGENT_CR3_SWITCH_OK` requires every PIT, Agent-call, #UD, and #GP
entry to observe its private Agent CR3 and every return to normal context to
restore the kernel CR3.
`AGENT_KERNEL_MULTI_AGENT_ISOLATION_OK` requires B's retained receive frame,
the read-only signal pages, and A's preempted then yielded frames to remain
intact across the opposite Worker's execution.
`AGENT_KERNEL_MULTI_AGENT_CONTEXT_SWITCH_OK` requires ten Agent-call CR3
transitions for A, ten for B, eight for the Verifier, and four for the restarted
Fault Worker; seven PIT round trips include A's live call session, the Verifier,
and all three Fault Worker generations, followed by Fault Worker #UD and #GP
returns, with all terminal scheduler states.
`AGENT_KERNEL_HETEROGENEOUS_AGENT_EXECUTION_OK`
requires the two different Worker call sequences, the Verifier sequence, and
the restarted Fault Worker sequence to return at their verified offsets with
different nonces.
`AGENT_KERNEL_UART_IRQ_OK` requires IRQ4 to capture one THRE interrupt and normal
context to validate its IIR/LSR mailbox. That signal causes event 104 and the
running Driver Invocation. The `O` in `AGENT_KERNEL_PORT_IO_BACKEND_OK` is
emitted by the immutable causal request at event 111 through the registered COM1
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
