# X86 Runtime Admission Requester Discovery V1 Design

## Status

Implemented and validated on 2026-07-19.

## Purpose

The resident Runtime Service Worker currently sends its completion `Notify` to
literal Agent 12. The Runtime Admission record already carries the authorized
requester, while that identity stops at the semantic broker boundary. A target
Capsule therefore cannot discover who admitted it and cannot remain reusable
under another Supervisor identity.

Requester Discovery V1 carries the kernel-owned requester into the admitted
CPU context and exposes it through a dedicated read-only Agent Call. The Worker
builds its notification recipient from the trusted reply and contains no fixed
Supervisor identity.

## ABI Operation 28

The stable V1 register ABI gains:

```text
AGENT_CALL_DISCOVER_RUNTIME_ADMISSION = 28
```

The request uses the standard authenticated context payload:

| Register | Request value |
| --- | --- |
| `rax` | `AGNTCALL` magic |
| `rbx` | ABI version 1 |
| `rcx` | operation 28 |
| `rdx` | zero flags |
| `rsi` | current Agent ID |
| `rdi` | current Task ID |
| `r8` | current Image ID |
| `r9` | established nonce |
| `r10-r15`, `rbp` | zero |

The successful reply preserves the standard common fields and returns:

| Register | Reply value |
| --- | --- |
| `rcx` | success status |
| `rdx` | operation 28 |
| `rsi`, `rdi`, `r8`, `r9` | trusted Agent, Task, Image, nonce |
| `r10` | kernel-owned Runtime Admission requester Agent ID |
| `r11-r15`, `rbp` | zero |

The existing `DescribeContext` reply keeps every reserved register at zero.
This preserves all earlier Capsules and ABI tests.

## Admitted Context Binding

`AgentCallContext` gains an optional Runtime Admission requester. Its regular
constructor creates a context with no requester. A separate admitted
constructor requires five nonzero identities:

- target Agent;
- target Task;
- verified Image;
- task-scoped Capability;
- requesting Supervisor Agent.

`NativeRuntimeAdmissionBroker` obtains the requester only from the
generation-bound semantic permit and creates the admitted context before the
physical runtime transaction. Context equality includes the requester, so CPU
ownership, waiting state, completed reports, and reclamation evidence preserve
the binding by value.

Operation 28 authenticates the common Agent/Task/Image/nonce tuple. Reply
encoding additionally requires a requester-bearing context. A regular native
context reaches no successful discovery reply.

## Dynamic Worker Flow

Each Runtime Service Worker executes this five-call transcript:

1. `DescribeContext` establishes the trusted identity and nonce;
2. `DiscoverRuntimeAdmission` returns the requester in `r10`;
3. the Worker validates the operation reply and saves the requester on its
   private user stack;
4. `SubmitTaskResult` records the bounded result;
5. the Worker restores the requester into the SendMessage recipient register,
   sends one `Notify` carrying its trusted Task ID, and executes
   `CompleteTask`.

Stack save/restore occurs between the second and fourth calls and adds no
kernel transition. The final operation order is:

```text
DescribeContext
DiscoverRuntimeAdmission
SubmitTaskResult
SendMessage
CompleteTask
```

Assembly symbols define all five return offsets. The frozen Worker artifact is:

- Capsule length: 238 bytes;
- code length: 206 bytes;
- return offsets: `[46, 75, 144, 175, 204]`;
- Capsule SHA-256: `4177fd1a4aae3344f582ffeff948abc66ae0221b7ea9ea0922ecb8b55c4d8cf2`;
- code SHA-256: `786b0890bc4acdf372f4005320dd116b833358eab08aed865b10034efbedd9ef`.

Fresh assembly matches the Rust static byte-for-byte. The release ELF contains
exactly one complete Worker Capsule. The unchanged 600-byte Admission
Supervisor Capsule also matches fresh assembly and occurs exactly once.

## Notification Evidence

For every Worker, the terminal proof requires:

- completed CPU context requester equals its Runtime Admission requester;
- discovery appears exactly once after the initial describe call;
- notification recipient equals the context requester;
- notification sender, Task payload, Image, nonce, and message kind retain the
  existing authenticated values;
- both notifications reach `Acknowledged` in FIFO request order;
- the Worker assembly source contains no fixed recipient instruction.

The resident Supervisor remains Agent 12 in the reference profile. That value
is an input to the semantic request and broker context, with no Worker Capsule
dependency.

## Deterministic Reference Profile

The kernel event sequence remains unchanged because discovery is read-only.

| Evidence | Count |
| --- | ---: |
| Runtime Service Worker Agent Calls | 10 |
| Runtime Service Worker address-space switches | 20 |
| Runtime Admission requester discoveries | 2 |
| Worker completion notifications | 2 |
| Kernel-selected dispatches | 30 |
| Physical quantum expiries | 13 |
| Ordered kernel Events | 275 |

Supervisor calls, all semantic state counts, physical ownership counts,
reclamation counts, and the final 66-frame zeroed pool remain unchanged.

## QEMU Evidence

The reference boot adds this per-Worker proof line:

```text
AGENT_KERNEL_AGENT_CALL_RUNTIME_ADMISSION_DISCOVERY_OK
```

The strict script requires exactly two occurrences, exact five-call Worker
transcripts, the existing dynamic notification records, 275 events, and the
terminal handoff in debug and release profiles.

## Failure And Atomicity

- Unknown operation values and nonzero flags fail decoding.
- Zero context identifiers and nonzero reserved registers fail decoding.
- Identity or nonce mismatch fails authentication before reply encoding.
- A context without an admission requester cannot encode a discovery reply.
- A zero requester cannot construct an admitted context.
- Broker failure retains the existing physical rollback behavior.
- Worker reply validation failure enters its terminal halt loop before result,
  notification, or completion mutation.
- Discovery performs no semantic store mutation and emits no kernel Event.

## Validation

- Red ABI/context tests first failed on the missing operation, context binding,
  and reply encoder.
- The strict QEMU contract first failed on the missing discovery marker.
- Focused contracts, the full workspace suite, and the host Supervisor passed.
- All five `no_std` library checks and the freestanding x86 binary check passed
  for `x86_64-unknown-none`.
- Workspace and freestanding Clippy passed with warnings denied while retaining
  the existing `too_many_arguments` baseline allowance.
- The precise host/allocation API scan was empty, and formatting passed.
- Strict debug and release QEMU each produced two discovery markers, the exact
  five-call Worker transcripts, 275 ordered Events, and the terminal handoff.
- Fresh assembly, Rust static, digest, return-offset, and release ELF audits
  passed for both Runtime Admission Capsules; each complete Capsule occurs once
  in the release ELF.
- Both README languages describe operation 28 and the deterministic evidence.

## Deferred Work

- repeated resident admission batches under one Supervisor control loop;
- explicit requester discovery for cancellation and fault disposition paths;
- release notification delivery to the requesting Supervisor;
- bounded Runtime Admission record compaction;
- dynamic page-table growth, SMP synchronization, PCID lifecycle, and hardware
  TLB shootdown.
