# Runtime Admission Capacity V1 Design

## Status

Implemented and validated on 2026-07-19.

## Purpose

Runtime Admission storage currently uses `[RuntimeAdmissionRecord; TASKS]`.
Request capacity and release-batch bounds also read `TASKS`, so Task storage and
the admission control plane cannot be configured independently.

This milestone gives Runtime Admission its own fixed-capacity generic across
the core, syscall facade, boot handoff, and x86 reference profile. It also lets
a Supervisor retry a physically rejected Task while retaining earlier terminal
records in the active audit window.

## Configuration Contract

`KernelCore`, `AgentKernel`, and `BootedKernel` gain a trailing const generic:

```text
const RUNTIME_ADMISSIONS: usize = TASKS
```

The parameter is trailing and defaulted so every existing type instantiation
keeps its current capacity and source-level meaning. Explicit configurations
can choose a smaller or larger Runtime Admission store without changing Task,
Run Queue, Event, or Agent capacities.

The core owns:

```text
[RuntimeAdmissionRecord; RUNTIME_ADMISSIONS]
```

The core and facade expose a read-only `runtime_admission_capacity()` value for
configuration evidence. A zero-capacity store is valid and fails requests with
`RuntimeAdmissionStoreFull` before indexing storage.

## Request And Retry Semantics

A `Requested` or `Admitted` record is live. At most one live record may refer
to a target Agent or Task.

`Rejected` and `Released` records are terminal. They retain ordered evidence
and consume configured admission capacity until prefix compaction removes them.
A terminal record does not block a later request for the same target and Task
when the Task remains eligible. In practice, physical rejection leaves the Task
accepted and allows a Supervisor retry with a new monotonic Admission ID.

Request validation order remains deterministic:

1. authenticate the Supervisor, Capability, target, Task, Image, and Resource;
2. reject a conflicting live target or Task;
3. reject an exhausted Runtime Admission store;
4. preflight one Event slot;
5. append the new request and advance the shared generation once.

When all slots contain terminal records, the next valid retry returns
`RuntimeAdmissionStoreFull`. Prefix compaction is the bounded mechanism that
returns those slots.

## Release Bound

`prepare_runtime_admission_release_batch` compares `COUNT` with
`RUNTIME_ADMISSIONS`.

- `COUNT == 0` returns `RuntimeAdmissionReleaseBatchEmpty`.
- `COUNT > RUNTIME_ADMISSIONS` returns
  `RuntimeAdmissionReleaseBatchTooLarge`.
- A batch larger than `TASKS` can pass the structural bound when the configured
  admission capacity allows it; normal record and release-readiness checks then
  decide the result.

The generation, duplicate-ID, Event preflight, verified-Task, idle-Agent, and
atomic commit contracts remain unchanged.

## Compaction Interaction

Terminal retry records preserve insertion order. Compaction continues to
remove only a contiguous terminal prefix, emits one
`RuntimeAdmissionCompacted` Event per removed record, shifts retained records
in FIFO order, and advances the shared generation once.

Example with `TASKS = 1` and `RUNTIME_ADMISSIONS = 3`:

1. request Admission 1 and reject it;
2. retry as Admission 2 and reject it;
3. retry as Admission 3 and reject it;
4. a fourth request reports `RuntimeAdmissionStoreFull`;
5. compact through Admission 2;
6. retry as Admission 4, leaving active records `[3, 4]` and one Task.

This proves independent capacity, retained terminal evidence, monotonic IDs,
slot reuse, and unchanged Task storage.

## X86 Reference Evidence

The freestanding reference profile uses:

| Capacity | Value |
| --- | ---: |
| Tasks | 12 |
| Runtime Admissions | 16 |

Boot validation checks the configured admission capacity and prints:

```text
AGENT_KERNEL_RUNTIME_ADMISSION_CAPACITY_OK
```

The strict QEMU script requires exactly one marker in Debug and Release runs.
The existing four Admission records, Agent Calls, address-space switches,
Capsule bytes, return offsets, and 328 ordered Events stay unchanged.

## Compatibility

- Existing generic argument order remains stable.
- Existing omitted capacity arguments resolve to `TASKS`.
- Agent Call operations and register layouts remain unchanged.
- Event kinds, ordering, and x86 semantic counts remain unchanged.
- No allocator, host API, userspace pointer, or variable-capacity container is
  introduced.

## Validation

- red core contract for three retained rejection attempts with one Task;
- red facade contract for explicit capacity forwarding;
- red strict QEMU requirement for the new capacity marker;
- default-capacity compatibility contract;
- focused Runtime Admission protocol and compaction tests;
- full workspace tests and host Supervisor run;
- formatting, shell syntax, `no_std`, and freestanding x86 checks;
- workspace and freestanding Clippy with warnings denied;
- strict Debug and Release QEMU with all 328 Events;
- frozen Supervisor Capsule hash, return-offset, and Release ELF occurrence
  audit.

## Validation Result

- The core contract retained three rejected Admissions with two configured
  Task slots and one created Task, rejected a fourth request at capacity,
  accepted a three-record release preflight structurally, compacted two prefix
  records, and allocated monotonic Admission ID 4.
- A zero-capacity store rejected a valid request without changing records or
  Events.
- Default core and facade types reported capacity equal to `TASKS`; an explicit
  facade type reported its independent value.
- The x86 profile reported Task capacity 12 and Runtime Admission capacity 16
  through exactly one `AGENT_KERNEL_RUNTIME_ADMISSION_CAPACITY_OK` marker.
- Strict Debug and Release QEMU completed all 328 ordered Events and reached
  `SUPERVISOR_HANDOFF_READY`.
- Full workspace tests, the 78-Event host Supervisor run, formatting, shell
  validation, `no_std` checks, and the freestanding x86_64 check passed.
- Workspace and freestanding Clippy passed with warnings denied while retaining
  the repository's structural allowance for existing `too_many_arguments`
  sites.
- Supervisor Capsule size remained 1,206 bytes and executable code remained
  1,174 bytes. SHA-256 values remained
  `7f39ab25f4d01de012556befc42b19f991e4ec60e0cacee464eb2b33d8908b4b`
  and `a6a120bb46b30988ea9fa4b160035bcc32670f6f8aaeb624de5854cda4ace0b7`.
- All 16 return offsets remained
  `[44, 82, 169, 247, 358, 395, 506, 572, 659, 737, 848, 885, 996, 1059,
  1143, 1172]`; the Release ELF contained one exact Capsule occurrence and one
  exact executable-code occurrence.

## Deferred Work

- Task-store retirement and reuse;
- long-running admission loops across recycled Task identities;
- durable Event export and checkpointed audit history;
- a named capacity configuration type that reduces const-generic surface area;
- dynamic admission storage after an allocator contract is designed.
