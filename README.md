# Agent Kernel

Agent Kernel is an early prototype for an agent-native operating system kernel.
It is not a Linux wrapper, shell agent, or POSIX-first compatibility layer.

The project starts from new OS primitives instead of POSIX compatibility:
agents, owned resources, resource lifecycle, capabilities, capability
attenuation, agent launch entries, runtime admission, typed intents, actions, observations,
agent executable image identity records, checkpoints, rollback, verification,
tasks, fixed-width task results, delegation, native mailbox IPC, task wait signals, task fault traps,
fault handlers, fault policies, memory cells, native object namespace entries,
driver bindings, device events, driver invocations, driver commands, agent
execution contexts, driver endpoint registries, HAL dispatch requests, and
event logs.

## Current Scope

- `agent-kernel-core`: no_std-friendly agent registry, agent image records, agent launch entries, runtime admission, agent execution contexts, owned resource creation, resource lifecycle, capability lifecycle, capability attenuation, action, observation, checkpoint, intent store, task store, fixed-width task results, lifecycle, FIFO run queue, mailbox IPC, task wait signals, task fault traps, fault handlers, fault policies, memory cells, object namespace entries, driver endpoint registry, driver bindings, device event lifecycle, driver invocation scheduling, driver command lifecycle, rollback, and event model.
- `agent-kernel`: no_std kernel facade with syscall-style methods over the core model.
- `agent-kernel-hal`: no_std device backend contract for executing immutable, kernel-authorized driver requests.
- `agent-kernel-boot`: no_std boot handoff boundary that seeds the kernel with a deterministic bootstrap flow and exposes trusted mutable architecture initialization.
- `agent-kernel-x86_64`: no_std x86_64 bootloader entry, native one-page Worker and Verifier Agent Image Capsule parsing with SHA-256 verification binding, three isolated Agent CR3 roots with same-address private pages, owned suspended CPU frames, physical PIT IRQ0 preemption/resume through a shared RSP0 stack, a versioned returning Agent Call ABI, audited task-result submission and inspection, target-scoped verification, task completion, one-shot UART IRQ4 ingress, and byte-wide Port I/O behind the privileged Driver boundary.
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
task wakeup, message send, message receive,
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
Driver Invocation, waiting on a signal, or faulted on a task. Agent images store kernel-owned
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
until an authorized signal emission wakes them back into the run queue.
`IntentId`, `TaskId`, and `MessageId` values are allocated by fixed-capacity kernel stores rather than
invented by the supervisor. `WaiterId` and `MemoryCellId` values are also kernel-allocated, and
memory recall writes an audit event before returning a value. Delegation derives
a task-scoped action capability for the assignee, so the supervisor does not
grant broad resource authority to complete delegated work. Revoking the source
capability that authorized delegation also invalidates the derived task-scoped
capability before future task authorization succeeds. Mailbox IPC stores typed
kernel object references instead of heap-allocated bytes or host transport
handles. Memory cells store fixed-width typed words instead of files, byte
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
validates the exact CPU return RIP; all other exception gates lead to
vector-specific deterministic failure stubs. Before that IDT becomes active,
the kernel installs a permanent GDT with ring-0/ring-3 segments and a long-mode
TSS whose RSP0 points at a dedicated 32 KiB privileged stack. The boot adapter
then registers and verifies two distinct Worker images and one Verifier image.
Each immutable native Capsule has a fixed 32-byte AgentOS header followed by at
most one code page; the no_std loader rejects unsupported format, architecture,
kind, or flags, zero or mismatched ABI and entry versions, noncanonical lengths,
reserved data, and out-of-range entry offsets. It computes SHA-256 across the
exact header and code bytes and requires equality with the verified kernel image
record before any task can dispatch. A bounded allocator then consumes only
BootInfo `Usable` frames and creates three distinct Agent P4 roots. All three
inherit identical supervisor-only kernel mappings, while each dedicated P4
index 128 contains private frames at the same virtual addresses: one read-only
executable Agent code page, one read-only/NX signal page, an unmapped guard page,
and four writable/NX stack pages. The kernel CR3 has no translation for any
Agent region, and all root/code/signal/stack physical frames are pairwise
disjoint.

Each code page is zeroed, filled only from its verified Capsule, and read back
through the supervisor physical alias before mapping. Verifier registration,
task delegation, its separate resource-scoped Verify capability, image
verification, launch, and acceptance occupy events 38 through 48. Event 49 then
dispatches Worker A. Every Agent enters its declared offset through a five-word
`iretq` frame at CPL3. Entry clears all general-purpose registers, selects that
Agent's CR3, and only then returns to the Agent. PIT IRQ0 performs a hardware
privilege transition to TSS RSP0; assembly saves all integer registers plus
RIP/CS/RFLAGS/user-RSP/user-SS, records the interrupted CR3, and restores the
kernel CR3 before touching normal kernel context. The kernel validates the
complete 160-byte frame, copies it into the preempted Agent context, and
releases RSP0 for the next Agent. Events 50 through 53 expire A and B in turn
and redispatch A with queue order preserved.

The kernel releases only A's read-only signal through its supervisor alias,
proves B remains blocked, and resumes A's owned frame. Both Workers perform
returning DescribeContext and SubmitTaskResult calls followed by a terminal
CompleteTask call. Only an exact scheduler-owned Agent/Task/Image identity and
nonce echo, matched to the kernel-held delegated task capability, can store a
fixed-width result while the task remains `Running`. A's result and completion
produce events 54 and 55; B dispatches at event 56 and produces its distinct
result and completion at events 57 and 58. A uses a 78-byte image, while B uses
an 80-byte image with a two-NOP prefix and a different nonce. Their
DescribeContext/SubmitTaskResult/CompleteTask return offsets are 46/67/76 and
48/69/78.

The kernel then queues and dispatches the Verifier at events 59 and 60, expires
it once at event 61, and redispatches it at event 62. Its first returning call
describes its trusted context. Its second call inspects only Worker A's stored
result under resource-scoped Verify authority and emits the audited
`TaskResultInspected` event 63 without mutating scheduler state. Ring-3 machine
code compares the returned words with `0x0a01` and `0xa11c0001`; a mismatch
enters a terminal loop. Reaching its third call therefore proves that the
comparison succeeded before Worker A becomes Verified at event 64 and its
intent becomes Fulfilled at event 65. The fourth call completes the Verifier's
own task at event 66. Worker B remains Completed with its different result and
bound intent as a target-scoping control. The Verifier uses a 111-byte image
with DescribeContext/InspectTaskResult/VerifyTask/CompleteTask return offsets
46/64/100/109. All three execution contexts finish Idle and the run queue is
empty. This is an Agent-native image format, call ABI, scheduler, and
verification lifecycle, not a POSIX process or syscall ABI.
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
completion, a general dynamic context store beyond the three boot Agents, page-table
teardown, PCIDs, SMP execution, context migration, fatal
exception recovery, error-code decoding, double-fault IST, a general IRQ
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
4. Allocate three distinct Agent P4 roots, inherit supervisor kernel mappings, and map disjoint fixed CPL3 code, signal, guard, and stack regions at identical virtual addresses.
5. Initialize `AgentKernel`.
6. Register the bootstrap agent.
7. Register a bootstrap resource.
8. Grant observe/act/verify/delegate capability to the bootstrap agent.
9. Register a bootstrap executable image as pending.
10. Verify that bootstrap image, moving it from pending to verified.
11. Launch the bootstrap agent into a bootstrap entry that references the verified image.
12. Record observation, action, and verification events.
13. Expose mutable handoff access for trusted architecture initialization.
14. Register a COM1 Port endpoint and admit a dedicated Driver Agent.
15. Register two admitted Worker Agents, create both delegated tasks, and enqueue them without dispatching.
16. Register a Verifier Agent with its own delegated task capability and a separate resource-scoped Verify capability.
17. Register and verify two distinct native Worker images plus one native Verifier image, bind each launch to its matching image kind, and accept the Verifier task without initially queuing it.
18. Parse all three fixed Capsule headers, bind their SHA-256 digests to the verified records, and copy/read back all three private code pages.
19. Dispatch A with quantum one, let IRQ0 switch to RSP0, copy the validated frame, expire A, and dispatch B.
20. Enter B under its distinct CR3, copy its IRQ0 frame, expire B, and redispatch A.
21. Resume A, answer DescribeContext, authorize and record its fixed-width TaskResult, return that successful mutation reply to ring 3, then validate CompleteTask, complete A, and dispatch B.
22. Prove A's signal and replies did not affect B, run B's distinct three-call round trip, and preserve both Worker results in Completed tasks.
23. Queue and dispatch the Verifier, preempt it once through IRQ0, and redispatch its owned frame under the third Agent CR3.
24. Return Worker A's result through an audited InspectTaskResult call, compare it in ring 3, verify only A, fulfill A's intent, and complete the Verifier's own task while B remains unverified.
25. Prove all three execution contexts are Idle and the run queue is empty.
26. Install IRQ4 in the persistent IDT, remap the PIC, arm COM1 THRE, and receive the hardware interrupt.
27. Validate the interrupt mailbox, raise and deliver an Interrupt Device Event, then dispatch and tick its Driver Invocation.
28. Acknowledge the event and dispatch a causally linked COM1 write request.
29. Record the write result, complete the Driver Invocation, and mark supervisor handoff ready.

The handoff now runs inside QEMU through the x86_64 BIOS image path.

## Non-Goals For V0

- Booting on physical hardware.
- UEFI image support.
- POSIX compatibility.
- Linux syscall compatibility.
- A filesystem, network stack, dynamic/SMP multi-Agent scheduler, or complete physical hardware driver stack.
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
AGENT_KERNEL_AGENT_B_PREEMPTION_OK
AGENT_KERNEL_TIMER_PREEMPTION_OK
AGENT_KERNEL_MULTI_AGENT_ISOLATION_OK
AGENT_KERNEL_AGENT_CPU_RESUME_OK
AGENT_KERNEL_AGENT_CALL_RESULT_OK
AGENT_KERNEL_AGENT_CALL_RETURNING_MUTATION_OK
AGENT_KERNEL_VERIFIER_PREEMPTION_OK
AGENT_KERNEL_AGENT_CALL_INSPECT_RESULT_OK
AGENT_KERNEL_AGENT_CALL_VERIFY_OK
AGENT_KERNEL_NATIVE_VERIFIER_OK
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
event[49] task_dispatched
event[50] task_quantum_expired
event[51] task_dispatched
event[52] task_quantum_expired
event[53] task_dispatched
event[54] task_result_submitted
event[55] task_completed
event[56] task_dispatched
event[57] task_result_submitted
event[58] task_completed
event[59] task_queued
event[60] task_dispatched
event[61] task_quantum_expired
event[62] task_dispatched
event[63] task_result_inspected
event[64] task_verified
event[65] intent_fulfilled
event[66] task_completed
event[67] device_event_raised
event[68] device_event_delivered
event[69] driver_invocation_queued
event[70] driver_invocation_dispatched
event[71] driver_invocation_ticked
event[72] device_event_acknowledged
event[73] driver_command_submitted
event[74] driver_command_dispatched
event[75] driver_command_completed
event[76] driver_invocation_completed
SUPERVISOR_HANDOFF_READY
```

`AGENT_KERNEL_GDT_TSS_OK` requires the permanent segment selectors and loaded
task register to match the host-tested GDT/TSS contract. The vector 3 trap gate
then captures the exact post-`int3` RIP and returns through `iretq` before
`AGENT_KERNEL_EXCEPTION_BASELINE_OK` is emitted.
`AGENT_KERNEL_AGENT_IMAGE_FORMAT_OK` requires two exact Worker Capsules and one
Verifier Capsule with bounded canonical x86_64 V0 headers.
`AGENT_KERNEL_AGENT_IMAGE_DIGEST_OK` requires the Workers' computed SHA-256
values, kind, status, ABI version, and entry version to match their distinct
verified kernel records. `AGENT_KERNEL_VERIFIER_IMAGE_OK` proves the same
binding for Capsule kind 2 and the verified Verifier record.
`AGENT_KERNEL_AGENT_USER_MEMORY_OK` requires successful fixed mappings for the
Agent RX code, read-only/NX signal, unmapped guard, and writable/NX stack pages.
`AGENT_KERNEL_AGENT_ADDRESS_SPACE_OK` additionally requires distinct aligned P4
roots, identical supervisor-only inherited mappings, an unused kernel P4 index
128, and no kernel translation for any Agent-owned virtual page.
`AGENT_KERNEL_VERIFIER_MEMORY_OK` and `AGENT_KERNEL_MULTI_AGENT_MEMORY_OK`
require three pairwise-disjoint physical memory identities behind the same
virtual layout and a common kernel root. `AGENT_KERNEL_AGENT_IMAGE_LOAD_OK`
additionally requires all three private code frames to match their verified
payload byte-for-byte before event 49 dispatches A.
`AGENT_KERNEL_PIT_IRQ_OK` requires the shared IDT's IRQ0 assembly entry to
capture exactly one PIT channel 0 interrupt after hardware switched from CPL3
to TSS RSP0. `AGENT_KERNEL_AGENT_CPU_PREEMPTION_OK` requires a complete
160-byte integer/privilege frame and a successful switch back to CPL0.
`AGENT_KERNEL_AGENT_RING3_PREEMPTION_OK` additionally requires exact user
selectors, in-range user RIP/RSP, IF set, IOPL clear, and an intact RSP0 canary.
`AGENT_KERNEL_AGENT_B_PREEMPTION_OK` additionally requires a second CR3 and a
second owned frame after RSP0 has been reused. `AGENT_KERNEL_TIMER_PREEMPTION_OK`
requires events 50 through 53 to preserve `[B, A]` then `[A, B]` FIFO rotation.
`AGENT_KERNEL_AGENT_CPU_RESUME_OK` requires both Worker PIT frames to resume at
their captured CPL3 RIP. `AGENT_KERNEL_AGENT_CALL_RESULT_OK` requires A and B to
persist `{0x0a01, 0xa11c0001}` and `{0x0b02, 0xb22c0002}` in events 54 and 57
without leaving `Running`. `AGENT_KERNEL_AGENT_CALL_RETURNING_MUTATION_OK`
additionally requires each semantic result event and unchanged scheduler state
to be validated before its success reply can resume ring 3.

`AGENT_KERNEL_VERIFIER_PREEMPTION_OK` requires the third Agent CR3 to survive a
PIT expiry and redispatch at events 61 and 62. The inspection marker requires
operation 5 to echo the Verifier's scheduler-owned context and target Worker A,
emit event 63, return the stored result in R10/R11, and leave both tasks and the
run queue unchanged. `AGENT_KERNEL_AGENT_CALL_VERIFY_OK` requires ring-3 code to
compare those words before operation 6 can verify only A and emit events 64 and
65. `AGENT_KERNEL_NATIVE_VERIFIER_OK` requires the fourth call to complete the
Verifier's own task at event 66, with all three contexts Idle and Worker B still
Completed but unverified.

`AGENT_KERNEL_AGENT_CALL_ABI_OK` requires three canonical `AGNTCALL` requests
per Worker and four per Verifier with version 1, supported operations, zero
flags, operation-specific registers, and zero reserved words. The return marker
requires DescribeContext, SubmitTaskResult, InspectTaskResult, and VerifyTask to
return trusted identity, nonce, and operation-specific payloads through owned
`iretq` frames before either terminal CompleteTask request.
`AGENT_KERNEL_AGENT_CALL_AUTHORITY_OK` requires each physical request to match
the scheduler context and kernel-held capability appropriate to that operation;
the Verifier's resource Verify capability never appears in ring-3 registers.
`AGENT_KERNEL_AGENT_CALL_COMPLETE_OK` requires core authorization to accept
those capabilities, produce Worker completion events 55 and 58 plus Verifier
completion event 66, and leave all three tasks in their expected terminal
states with an empty run queue. `AGENT_KERNEL_AGENT_CR3_SWITCH_OK` requires each
Agent's PIT and Agent-call entry to observe its private CR3 and every return to
normal context to restore the kernel CR3.
`AGENT_KERNEL_MULTI_AGENT_ISOLATION_OK` requires A's released signal to leave B
blocked. `AGENT_KERNEL_MULTI_AGENT_CONTEXT_SWITCH_OK` requires six physical
Agent-call CR3 transitions per Worker, eight for the Verifier, and all terminal
scheduler states. `AGENT_KERNEL_HETEROGENEOUS_AGENT_EXECUTION_OK` requires the
two Worker call sequences and the distinct Verifier call sequence to return at
their verified offsets with different nonces.
`AGENT_KERNEL_UART_IRQ_OK` requires IRQ4 to capture one THRE interrupt and normal
context to validate its IIR/LSR mailbox. That signal causes event 67 and the
running Driver Invocation. The `O` in `AGENT_KERNEL_PORT_IO_BACKEND_OK` is
emitted by the immutable causal request at event 74 through the registered COM1
endpoint and `PortIoBackend`; the surrounding text uses the existing boot
serial writer. The remaining success markers are printed only after the write
and Driver Invocation terminal records are verified as `Completed` and the
Driver execution context is `Idle`.

The x86 entry configures a 512 KiB kernel stack plus a separate 32 KiB TSS RSP0
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
