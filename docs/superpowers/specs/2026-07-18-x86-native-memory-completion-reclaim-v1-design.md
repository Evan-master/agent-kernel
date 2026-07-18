# X86 Native Memory Completion Reclaim V1 Design

## Status

Implemented and validated on 2026-07-18.

## Purpose

Fault Reclaim V1 gives the exception path deterministic ownership of live
private runtime memory. Successful native Task completion still requires every
Agent to issue explicit page and region release calls before `CompleteTask`.
An omitted release currently terminates the boot proof and leaves the Task in
its running state.

Completion Reclaim V1 applies the same bounded cleanup transaction to an
authenticated completion request. The kernel validates Task completion and all
Memory retirement authority before mutation, retires every live Memory
Resource, removes private leaves, clears and returns physical frames, attaches
ordered cleanup evidence to the completed CPU, and then commits
`TaskCompleted`.

## Completion Readiness

The core and facade expose a read-only `can_complete_task` check. It validates:

- active Agent identity;
- matching `Act` authority, exact scope for task-scoped Capabilities, and the
  complete parent chain;
- running Task status and matching assignee;
- runtime admission for the Agent and Task;
- one available event slot.

`complete_task` delegates its validation to this method before changing Task or
execution-context state. The native completion adapter calls the same public
facade check before runtime-memory cleanup, which prevents an invalid
completion request from consuming mappings or Resources.

## Shared Reclamation Transaction

The fault and completion paths use one fixed-capacity transaction engine. A
private owner adapter supplies the captured CPU type while preserving the
existing layer boundary:

- architecture-library ledgers provide a page-first, deterministic plan;
- the executor validates semantic records, exact Capability authority, private
  leaves, pool ownership, generation tokens, and physical proof words;
- Resource retirement runs through `sys_retire_resource`;
- Agent-memory methods remove leaves and commit private ledgers;
- the global pool clears and returns every frame;
- the caller owns lifecycle-specific evidence attachment and terminal mutation.

The transaction reserves one event slot per live Memory Resource plus one slot
for `TaskCompleted`. Single-core execution keeps the prepared plan stable during
commit. Any invariant failure follows the existing fail-closed path.

## Completion Evidence

`CompletedAgentCpu` gains a fixed-capacity `RuntimeReclamationLog`. An explicit
release flow carries an empty log. Automatic completion cleanup records one
entry per reclaimed mapping with:

- page or region kind;
- Resource, Capability, and MemoryCell identity;
- page count and allocation generation;
- first and last physical proof values.

The completion conversion accepts the log only after private ledgers are clear.
Terminal evidence also requires no global pool ownership for the Agent and all
returned frames fully zeroed.

## Resource Manager Proof

The ring-3 Resource Manager keeps Region C live when it submits its result and
requests completion:

| Field | Expected value |
| --- | ---: |
| Resource | 7 |
| Capability | 19 |
| MemoryCell | 5 |
| Pages | 3 |
| Generation | 3 |
| First proof | `0x524547494f4e4331` |
| Last proof | `0x524547494f4e4333` |

The Capsule omits the final `ReleaseMemoryRegion` call. Its authenticated
transcript decreases from 30 calls to 29 and from 60 address-space switches to
58. The final sequence becomes:

```text
InspectMemoryRegion(C)
    -> ReleaseMemoryRegion(B)
    -> SubmitTaskResult
    -> CompleteTask with C still mapped
```

The completion path retires Resource 7 immediately before `TaskCompleted` and
emits `AGENT_KERNEL_NATIVE_COMPLETION_MEMORY_RECLAIMED_OK` exactly once.

## Release Capsule Artifact

The release ELF contains the exact audited Capsule artifacts:

| Capsule | Capsule bytes | Code bytes | Agent Calls | Address-space switches | SHA-256 |
| --- | ---: | ---: | ---: | ---: | --- |
| Resource Manager | 2,594 | 2,562 | 29 | 58 | `ac5e435801817f5e39debf751ac360999d5e6c0c8e7423e8ceb09c3c1304d6fc` |
| Fault Worker | 325 | 293 | 4 | 8 | `a74bdafa93cb878d578b2dd75ff9b6000d0f6e96ab39d01d658496821aedc4de` |

The extracted Resource Manager code matches the assembled source byte for
byte. Disassembly exposes 29 `int 0x90` boundaries at the expected offsets and
ends in the fixed halt loop at code offset `0x0a00`.

## Deterministic Event Contract

Object capacities and final totals remain seven Resources, nineteen
Capabilities, five MemoryCells, and 205 Events. The final Manager events become:

```text
event[191] memory_cell_recalled
event[192] resource_retired
event[193] task_result_submitted
event[194] resource_retired
event[195] task_completed
```

The second retirement is the completion-time cleanup of Resource 7. Explicit
`ReleaseMemoryRegion` marker count decreases from three to two; allocation and
inspection counts remain four and three.

## Validation

Milestone completion requires:

- red and green core/facade completion-readiness tests;
- architecture and workspace tests;
- Supervisor execution;
- `no_std` and bare-metal checks;
- warning-free scoped Clippy;
- strict debug and release QEMU runs with 205 exact events;
- one completion-reclamation marker and one Fault-reclamation marker;
- release ELF Capsule extraction, SHA-256 verification, source-byte comparison,
  and x86_64 disassembly inspection;
- public `main` publication and remote hash verification.

## Deferred Work

- memory cleanup for externally cancelled running Tasks;
- cleanup for suspended or retired Agents with retained mappings;
- complete private address-space destruction;
- dynamic page-table intermediate allocation;
- SMP synchronization and hardware TLB shootdown.
