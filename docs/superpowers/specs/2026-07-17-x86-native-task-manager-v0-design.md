# X86 Native Task Manager V0 Design

## Status

Implemented, validated in debug and release QEMU, and published to public
`main` on 2026-07-17 in commit `ad4b93f`.

## Purpose

The x86 runtime already lets ring-3 Agents complete work, exchange messages,
manage child resources, and control derived capabilities. Intent declaration,
Task creation, and Task delegation still run during trusted bootstrap. Native
Task Manager V0 exposes those existing kernel primitives through the
register-only Agent Call ABI and exercises them from the kind-4 Manager Capsule.

The resulting lifecycle is:

```text
ring-3 Manager declares an Act Intent
    -> kernel allocates Intent 7 and records IntentDeclared
    -> Manager creates Task 7 from Intent 7
    -> kernel records TaskCreated and IntentBound
    -> Manager delegates Task 7 to registered Agent 2
    -> kernel derives task-scoped Capability 13
    -> kernel records CapabilityDerived and DelegationRequested
    -> Manager reports the returned handles and completes its own Task
```

## Layer Placement

- `agent-kernel-core` retains the existing fixed-capacity Intent and Task
  lifecycle semantics.
- `agent-kernel` remains the only mutation facade used by architecture code.
- The host-testable x86 ABI owns operations 14 through 16, strict register
  decoding, scheduler-context authentication, and canonical replies.
- The bare-metal executor invokes facade methods and validates every returned
  object and event before resuming ring 3.
- The kind-4 Manager Capsule issues the native protocol and validates each
  reply in machine code.
- The Resource Manager evidence module binds final objects, event order,
  capability lineage, and the exact call transcript.

## Authority Model

Bootstrap derives one resource-scoped capability for Agent 8 with `Act` and
`Delegate` over the bootstrap Workspace. The Manager uses this capability as
the explicit authority argument for all three operations.

- Intent declaration requires the operation selected by `IntentKind`.
- Task creation requires ownership of the declared Intent and the same
  resource operation.
- Task delegation requires `Act` and `Delegate`, an active target Agent, and a
  Task in `Created` state.
- Delegation derives one `Act` capability scoped to the new Task.

The running Manager context continues to authenticate with its own task-
scoped Capability 9. Supplying an authority handle in a request grants no
ambient access; the public facade and core validate ownership and scope.

## Agent Call ABI

All requests use ABI magic `AGNTCALL`, version 1, flags 0, and the authenticated
identity tuple in `rsi`, `rdi`, `r8`, and `r9`:

| Register | Meaning |
| --- | --- |
| `rsi` | calling Agent |
| `rdi` | running Task |
| `r8` | loaded Agent Image |
| `r9` | non-zero session nonce |

### Operation 14: DeclareIntent

| Register | Request value |
| --- | --- |
| `r10` | authority Capability |
| `r11` | Resource |
| `r12` | Intent kind code |
| `r13` | verification requirement code |
| `r14`, `r15`, `rbp` | zero |

Intent kind codes are `1 Observe`, `2 Act`, `3 Verify`, `4 Checkpoint`, and
`5 Rollback`. Verification codes are `1 Optional` and `2 Required`.
The success reply returns the allocated Intent in `r10`.

### Operation 15: CreateTask

| Register | Request value |
| --- | --- |
| `r10` | authority Capability |
| `r11` | declared Intent |
| `r12`, `r13`, `r14`, `r15`, `rbp` | zero |

The success reply returns the allocated Task in `r10`.

### Operation 16: DelegateTask

| Register | Request value |
| --- | --- |
| `r10` | authority Capability |
| `r11` | created Task |
| `r12` | target Agent |
| `r13`, `r14`, `r15`, `rbp` | zero |

The success reply returns the Task in `r10`, the task-scoped Capability in
`r11`, and the target Agent in `r12`.

Every decoder rejects zero handles, unknown enum codes, oversized enum values,
unsupported operation values, and non-zero reserved registers. Replies clear
all unrelated payload registers and preserve the privilege-frame control
words.

## Native Capsule Protocol

The Manager Capsule retains its resource and capability lifecycle, then adds
the three Task Manager calls:

```text
DescribeContext
CreateResource
DeriveCapability
RevokeDerivedCapability
RetireResource
DeclareIntent
CreateTask
DelegateTask
SubmitTaskResult
CompleteTask
```

The Manager validates status and operation fields after every returning call.
Its final result uses code `0xc003` and packs Intent 7, Task 7, and Capability
13 into value `0x000000070007000d`.

The immutable Capsule contains a 32-byte header and 606 bytes of code for a
total of 638 bytes. Its SHA-256 digest is
`676c6dbf4575490ea6f19e092bc0e6b9db45ffde58699981bae0a3ae0a46e64d`.
The exact return offsets are
`[45, 86, 163, 236, 310, 390, 463, 527, 595, 604]`.

## Deterministic Capacity Changes

- Capabilities: 12 to 13.
- Intents: 6 to 7.
- Tasks: 6 to 7.
- Events: 171 to 176.
- Per-session Agent Call transcript entries: 8 to 10.
- Agents, Resources, run queue entries, images, and other stores remain at
  their current capacities.

The five new events occur after `ResourceRetired` and before the Manager's
`TaskResultSubmitted` event:

```text
IntentDeclared
TaskCreated
IntentBound
CapabilityDerived
DelegationRequested
```

## Validation Contract

The milestone requires:

- focused red/green ABI contract tests;
- full workspace tests and Supervisor execution;
- x86 `no_std` checks and scoped Clippy;
- debug and release QEMU runs with exactly 176 events;
- exact Capsule digest and return-offset validation;
- release ELF Capsule extraction and disassembly inspection;
- final state checks for Intent 7, Task 7, Capability 13, and Agent 2;
- unchanged physical scheduling counts outside the five semantic events.

## Scope Boundary

This milestone creates and delegates a Task to an existing Agent. Native Agent
registration, image installation, image verification, runtime memory
allocation, and launch admission remain separate milestones.
