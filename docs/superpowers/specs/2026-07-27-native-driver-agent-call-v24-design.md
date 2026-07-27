# Native Driver Agent Call V24 Design

**Status:** implemented and verified

## Objective

V24 moves PCI serial command selection from the boot adapter into a native
ring-3 Driver Agent. Core remains the authority owner, the x86 runtime remains
the privilege-transition owner, and the HAL remains the only layer that can
execute physical port I/O.

```text
Device Event
  -> Driver Invocation
  -> ring-3 Driver image
  -> authenticated Agent Calls
  -> immutable Driver Command
  -> ring-0 HAL dispatch
  -> physical PCI serial byte
```

The Driver Agent receives semantic identities and event data. It never receives
the PCI BDF, BAR index, I/O base, or an unrestricted port primitive.

## Native Driver Image

The x86 Agent Image format gains image-kind wire value `6` for
`AgentImageKind::Driver`. Verification binds the exact Capsule digest and
metadata to the Core image record before any mapping or execution.

The V24 PCI serial Driver Capsule is one page or smaller and performs exactly:

1. describe the current execution context;
2. inspect its Driver Invocation and Device Event;
3. acknowledge that Device Event;
4. submit one `Write` command with opcode `0` and value `0x50`;
5. validate the hardware result and complete the Driver Invocation.

An adjacent assembly source is the auditable byte source. The image audit must
prove an exact match between assembled `.text` and embedded Capsule code.

## Execution Context

`AgentCallContext` gains a mutually exclusive Driver scope:

```text
Task scope    TaskId != 0, DriverInvocationId absent
Driver scope  TaskId == 0, DriverInvocationId != 0
```

Existing Task constructors, replies, and wire values stay byte-compatible.
For a Driver context, the common reply payload is:

```text
rsi  AgentId
rdi  DriverInvocationId
r8   AgentImageId
r9   session nonce
r10  context kind = 1
```

Task-context replies retain `r10 = 0`. Task-only request authentication rejects
a Driver context, and Driver-only authentication rejects a Task context.

## Driver Agent Call ABI

V24 extends ABI version 1 without changing operations `1..56`.

### 57: Inspect Driver Invocation

Request:

```text
rsi  AgentId
rdi  DriverInvocationId
r8   AgentImageId
r9   nonce
r10..rbp  zero
```

Reply:

```text
r10  DeviceEventId
r11  ResourceId
r12  DriverBindingId
r13  DeviceEventKind
r14  payload.code
r15  payload.value
rbp  zero
```

Only a currently running Invocation whose Driver, image entry, Capability,
Resource, Binding, and Event agree with the scheduler-owned context can be
inspected.

### 58: Acknowledge Device Event

Request adds `r10 = DeviceEventId`; other extended registers are zero. Core
performs the existing `Observe + Act` authority and running-context checks.
The reply returns the acknowledged Event identity in `r10`.

### 59: Submit Driver Command

Request:

```text
r10  DeviceEventId
r11  DriverCommandKind
r12  opcode
r13  value
r14..rbp  zero
```

V24 defines wire kind `1` as `DriverCommandKind::Write`. Resource and
Capability are taken from the trusted context and Invocation record. Core
creates and dispatches an immutable command. The architecture backend executes
the returned request and records the typed result before replying:

```text
r10  DriverCommandId
r11  result.code
r12  result.value
r13..rbp  zero
```

### 60: Complete Driver Invocation

The request carries only the common Driver identity. Completion requires an
acknowledged Device Event and a completed command bound to the same Invocation.
The call is terminal: the CPU owner transfers to the completion report without
returning to ring 3.

## Native Scheduling

`NativeAgentRuntime` keeps the existing fixed-capacity owner registry. Driver
dispatch uses the Core FIFO selector and then takes only a parked CPU whose
Agent and `DriverInvocationId` match that committed selection.

The initial ring-3 release loop is preempted by the physical quantum. The
runtime records `DriverInvocationQuantumExpired`, parks the full frame, and
re-dispatches the same queued Invocation. This proves that the Driver path uses
the native scheduler and resumable privilege frame rather than a direct
function call.

Further physical preemptions repeat the same bounded transition. Unsupported
Agent Calls, identity mismatch, and invalid Core state fail closed.

## Completion And Reclamation

The Driver image has no runtime-memory operations. Terminal completion requires
all runtime-memory ledgers to be clear. The completed CPU enters the existing
address-space completion report, then the BSP:

1. removes the private Agent mapping;
2. performs the SMP TLB shootdown;
3. clears every private content and page-table frame;
4. returns the exact frame identity to the sealed address-space pool.

Pool size and all-zero state must return to their pre-admission values.

## PCI Serial Boundary

The existing `PciSerialBackend` remains unchanged. It is constructed from the
Core-owned endpoint produced by the exact V22 claim. The Driver Agent submits
only:

```text
kind    Write
opcode  0
value   0x50
```

Ring 0 validates that Core's dispatched request matches the current
Invocation, then passes it to the backend. Physical `OUT` occurs only after
the backend observes THRE within the fixed poll budget.

## Event Contract

V24 produces the exact Event history `1..435`. The native PCI Driver suffix is:

```text
418  agent_image_retired
419  agent_image_record_retired
420  capability_derived
421  agent_image_registered
422  agent_image_verified
423  agent_launched
424  driver_bound
425  device_event_raised
426  device_event_delivered
427  driver_invocation_queued
428  driver_invocation_dispatched
429  driver_invocation_quantum_expired
430  driver_invocation_dispatched
431  device_event_acknowledged
432  driver_command_submitted
433  driver_command_dispatched
434  driver_command_completed
435  driver_invocation_completed
```

Agent Call transcripts provide the separate proof that operations
`DescribeContext`, `InspectDriverInvocation`, `AcknowledgeDeviceEvent`,
`SubmitDriverCommand`, and `CompleteDriverInvocation` executed in ring 3.

## Failure Semantics

- Driver Capsules with a non-Driver image kind fail metadata verification.
- Driver context construction rejects zero identities and Task/Driver overlap.
- Driver requests with wrong Agent, Invocation, Image, nonce, or reserved
  registers fail before mutation.
- Task requests cannot authenticate under a Driver context.
- Driver requests cannot authenticate under a Task context.
- Event acknowledgement requires the context's running Invocation.
- Command submission derives Resource and Capability from trusted state.
- Backend failure is recorded as a failed command and produces no success
  reply.
- Completion requires one coherent acknowledged Event and completed Command.
- Any failed physical ownership transfer stops boot before supervisor handoff.

## Deferred Work

- Driver fault containment and restart policy;
- PCI INTx routing into the new Driver runtime;
- multiple outstanding commands per Invocation;
- MMIO, MSI, MSI-X, DMA, and IOMMU domains;
- hotplug and Driver replacement.

## Verification

V24 verification covers:

- Driver image-kind parser and metadata tests;
- all four Driver Agent Call decode, reserved-register, authentication, and
  canonical-reply contracts;
- Task/Driver context isolation tests;
- native Driver dispatch ownership tests where practical;
- exact image assembly and digest audit;
- exact ring-3 transcript and reclaimed-frame evidence;
- exact PCI serial byte `0x50`;
- workspace tests, host and bare-metal strict Clippy, supervisor, shell,
  debug QEMU, and release QEMU gates.

The published implementation passes all listed gates. The image audit verifies
9 native images, 2 signed Package v3 images, 5 exact assembly sources, and
unique Package, Capsule, code, and rodata bytes in the Release ELF.
