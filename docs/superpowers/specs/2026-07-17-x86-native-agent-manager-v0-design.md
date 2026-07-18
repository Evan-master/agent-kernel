# X86 Native Agent Manager V0 Design

## Status

Implemented, validated in debug and release QEMU, and published to public
`main` on 2026-07-17 in commit `cc08186`.

## Purpose

Native Resource, Capability, Intent, and Task lifecycle operations now cross
the ring-3 Agent Call boundary. Agent identity registration and status changes
still use trusted bootstrap methods with no actor or Capability in their
contracts. Native Agent Manager V0 adds a resource-scoped management domain to
new Agent records and exposes a bounded registration lifecycle to the running
Manager Capsule.

The lifecycle is:

```text
ring-3 Manager registers Agent 9 under Workspace 1
    -> Delegate authority and root scope are validated
    -> Agent 9 receives an idle execution context
    -> Manager suspends Agent 9
    -> Manager resumes Agent 9
    -> Manager retires Agent 9
    -> four ordered lifecycle events retain actor, target, Resource, Capability
```

## Agent Management Domain

`AgentRecord` gains two optional fields:

- `manager`: the Agent that created the managed identity;
- `management_resource`: the Resource whose root-scoped `Delegate` authority
  governs lifecycle operations.

Trusted bootstrap registration stores `None` in both fields and retains its
existing API and event contract. Managed registration stores both values and
records the Manager as event actor, the new identity as `target_agent`, and the
authority Resource and Capability on the event.

Any active Agent holding root-scoped `Delegate` authority on the stored
management Resource may perform later lifecycle transitions. This supports
delegated administration while preserving the creator in the immutable Agent
record.

## Safety Conditions

Managed suspend, resume, and retire operations require:

- a non-zero caller and target identity;
- distinct caller and target Agents;
- an active caller;
- a target created through managed registration;
- an active management Resource;
- a root-scoped, active `Delegate` Capability owned by the caller;
- an idle target execution context;
- no launch entry for the target;
- no non-terminal Task assigned to the target;
- sufficient event capacity before state mutation.

The trusted lifecycle methods remain available to bootstrap and host tests.
The managed methods are the only lifecycle entrypoints exposed through Agent
Call operations.

## Layer Placement

- `agent-kernel-core` owns Agent management metadata, authorization, quiescence
  checks, state mutation, and event construction.
- `agent-kernel` exposes managed lifecycle methods through a dedicated facade
  module.
- The host-testable x86 ABI owns operations 17 through 20, strict register
  decoding, context authentication, and canonical replies.
- The bare-metal executor invokes public facade methods and verifies exact
  records and events before resuming ring 3.
- The kind-4 Manager Capsule performs the lifecycle and validates all replies.
- Resource Manager evidence binds final Agent state, call transcript, and event
  order to the existing physical execution evidence.

## Agent Call ABI

All requests use the existing ABI magic, version, zero flags, and authenticated
Agent, Task, Image, and nonce tuple.

### Operation 17: RegisterManagedAgent

| Register | Request value |
| --- | --- |
| `r10` | management Capability |
| `r11` | management Resource |
| `r12` | new Agent |
| `r13`, `r14`, `r15`, `rbp` | zero |

### Operations 18-20: Managed Lifecycle

| Operation | ID |
| --- | ---: |
| `SuspendManagedAgent` | 18 |
| `ResumeManagedAgent` | 19 |
| `RetireManagedAgent` | 20 |

Each lifecycle request places the management Capability in `r10`, the target
Agent in `r11`, and zero in `r12` through `rbp`.

Every success reply returns the target Agent in `r10`, its management Resource
in `r11`, and the resulting status code in `r12`. Status codes are `1 Active`,
`2 Suspended`, and `3 Retired`. All unrelated payload registers are zero.

Decoders reject zero handles, unknown operations, unsupported flags, and
non-zero reserved registers. Scheduler-owned context authentication occurs
before any facade call.

## Native Capsule Protocol

The Manager Capsule extends its current ten-call protocol:

```text
DescribeContext
CreateResource
DeriveCapability
RevokeDerivedCapability
RetireResource
DeclareIntent
CreateTask
DelegateTask
RegisterManagedAgent
SuspendManagedAgent
ResumeManagedAgent
RetireManagedAgent
SubmitTaskResult
CompleteTask
```

The final result uses code `0xc004` and value `0x000900070007000d`, which packs
Agent 9, Intent 7, Task 7, and Capability 13.

The assembled Capsule contains a 32-byte header and 945 bytes of code for a
total of 977 bytes. Its SHA-256 digest is
`2620332b69be753aef4840550e21c78769ad28a4346790a2b82af7202df368bf`.
The 14 return offsets are `45`, `86`, `163`, `236`, `310`, `390`, `463`,
`539`, `626`, `710`, `794`, `866`, `934`, and `943`.

## Deterministic Capacity Changes

- Agents: 8 to 9.
- Events: 176 to 180.
- Per-session Agent Call transcript entries: 10 to 14.
- All other kernel stores retain their current capacities.

The four new events occur after `DelegationRequested` and before the Manager's
result and completion events:

```text
AgentRegistered
AgentSuspended
AgentResumed
AgentRetired
```

## Validation Contract

The milestone requires:

- red/green core tests for authorization, metadata, quiescence, atomicity, and
  all lifecycle states;
- facade tests for the public managed boundary;
- host ABI tests for operations 17 through 20 and canonical replies;
- exact final state for retired Agent 9 and its idle execution context;
- debug and release QEMU runs with exactly 180 events;
- exact Capsule digest, return offsets, release ELF extraction, and
  disassembly inspection;
- full workspace tests, Supervisor execution, no-std checks, and scoped Clippy.

## Scope Boundary

This milestone manages unlaunched Agent identities. Native image registration,
image verification, address-space allocation, launch admission, runtime
installation, and lifecycle coordination for running Agents remain separate
milestones.
