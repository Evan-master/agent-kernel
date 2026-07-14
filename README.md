# Agent Kernel

Agent Kernel is an early prototype for an agent-native operating system kernel.
It is not a Linux wrapper, shell agent, or POSIX-first compatibility layer.

The project starts from new OS primitives instead of POSIX compatibility:
agents, owned resources, resource lifecycle, capabilities, capability
attenuation, agent launch entries, runtime admission, typed intents, actions, observations,
agent executable image identity records, checkpoints, rollback, verification,
tasks, delegation, native mailbox IPC, task wait signals, task fault traps,
fault handlers, fault policies, memory cells, native object namespace entries,
driver bindings, device events, driver invocations, driver commands, agent
execution contexts, driver endpoint registries, HAL dispatch requests, and
event logs.

## Current Scope

- `agent-kernel-core`: no_std-friendly agent registry, agent image records, agent launch entries, runtime admission, agent execution contexts, owned resource creation, resource lifecycle, capability lifecycle, capability attenuation, action, observation, checkpoint, intent store, task store, lifecycle, FIFO run queue, mailbox IPC, task wait signals, task fault traps, fault handlers, fault policies, memory cells, object namespace entries, driver endpoint registry, driver bindings, device event lifecycle, driver invocation scheduling, driver command lifecycle, rollback, and event model.
- `agent-kernel`: no_std kernel facade with syscall-style methods over the core model.
- `agent-kernel-hal`: no_std device backend contract for executing immutable, kernel-authorized driver requests.
- `agent-kernel-boot`: no_std boot handoff boundary that seeds the kernel with a deterministic bootstrap flow and exposes trusted mutable architecture initialization.
- `agent-kernel-x86_64`: no_std x86_64 bootloader entry plus a bounded byte-wide Port I/O backend and kernel-dispatched COM1 command flow with native `in`/`out` instructions.
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
boot path registers and launches a dedicated Driver Agent, submits and dispatches
the physical COM1 request through the kernel, executes only that immutable
request, and records the result as a terminal kernel transition. MMIO, page
mapping, interrupt routing, wider port operations, and DMA policy remain future
architecture work.
Owner-aware resource creation assigns `owner: Some(agent)` and creates the
first capability atomically with the resource. Bootstrap `register_resource`
remains available for system-seeded resources and leaves `owner: None`.

## Boot Handoff

`agent-kernel-boot` currently validates the kernel-native boot contract:

1. Enter kernel phase.
2. Initialize `AgentKernel`.
3. Register the bootstrap agent.
4. Register a bootstrap resource.
5. Grant observe/act/verify/delegate capability to the bootstrap agent.
6. Register a bootstrap executable image as pending.
7. Verify that bootstrap image, moving it from pending to verified.
8. Launch the bootstrap agent into a bootstrap entry that references the verified image.
9. Record observation, action, and verification events.
10. Expose mutable handoff access for trusted architecture initialization.
11. On x86_64, register a COM1 Port endpoint and admit a dedicated Driver Agent.
12. Submit and dispatch a bounded write through the kernel command state machine.
13. Execute the immutable request with native Port I/O and record its terminal result.
14. Mark the kernel ready for supervisor handoff.

The handoff now runs inside QEMU through the x86_64 BIOS image path.

## Non-Goals For V0

- Booting on physical hardware.
- UEFI image support.
- POSIX compatibility.
- Linux syscall compatibility.
- A filesystem, network stack, preemptive scheduler, or complete physical hardware driver stack.
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

Expected QEMU serial output:

```text
AGENT_KERNEL_QEMU_BOOT_OK
AGENT_KERNEL_PORT_IO_BACKEND_OK
AGENT_KERNEL_PORT_COMMAND_FLOW_OK
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
event[16] driver_command_submitted
event[17] driver_command_dispatched
event[18] driver_command_completed
SUPERVISOR_HANDOFF_READY
```

The `O` in `AGENT_KERNEL_PORT_IO_BACKEND_OK` is emitted by the immutable request
returned from kernel command dispatch through the registered COM1 endpoint and
`PortIoBackend`; the surrounding text uses the existing serial writer. The
command-flow marker is printed only after the backend result is accepted by the
terminal syscall and the kernel command record is verified as `Completed`.

The x86 entry configures a 256 KiB kernel stack. The fixed-capacity kernel state
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
