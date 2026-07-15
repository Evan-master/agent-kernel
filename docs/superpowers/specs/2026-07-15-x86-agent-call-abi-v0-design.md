# X86 Agent Call ABI V0 Design

## Status

Implemented and verified locally on 2026-07-15. Publication remains pending
because the configured GitHub CLI credential is invalid.

## Purpose

The x86 runtime currently treats every DPL3 interrupt `0x90` as an implicit
task yield. The saved register frame is inspected only for one signal-page
address, so the boundary has no version, operation discriminator, arguments,
return value, or proof that execution resumed after a kernel response.

This milestone defines a native AgentOS call ABI. Each Worker first asks the
kernel to describe its trusted execution context, receives the bound
Agent/Task/Image identity, resumes in ring 3, and then echoes that response in a
structured Yield call. No POSIX syscall numbers, process identity, file
descriptors, or compatibility conventions are introduced.

## Register Envelope

The complete request is carried by the privilege interrupt frame already saved
by the `0x90` gate. V0 assigns registers as follows:

| Register | Request meaning |
| --- | --- |
| `RAX` | magic `0x4c4c4143544e4741` (`AGNTCALL` as little-endian bytes) |
| `RBX` | ABI version, exactly `1` |
| `RCX` | operation: `1` DescribeContext, `2` Yield |
| `RDX` | flags, exactly `0` |
| `RSI` | payload word 0 |
| `RDI` | payload word 1 |
| `R8` | payload word 2 |
| `R9` | payload word 3 |
| `R10` | reserved, exactly `0` |
| `R11` | reserved, exactly `0` |

Unknown magic, version, operation, flags, or reserved words fail closed before
semantic state can advance. The ABI decoder is a no_std architecture-library
module and performs no privileged operation.

## DescribeContext

DescribeContext accepts one nonzero caller nonce in `RSI`; `RDI`, `R8`, and
`R9` must be zero. The kernel never accepts identity claims in this request.
Instead, `PreparedAgentCpu` owns an `AgentCallContext` constructed from the
already-admitted scheduler tuple:

- the running `AgentId`,
- its delegated `TaskId`,
- the verified `AgentImageId` bound to its launch.

Success rewrites an owned copy of the saved frame with:

| Register | Reply meaning |
| --- | --- |
| `RAX` | ABI magic |
| `RBX` | ABI version |
| `RCX` | status `0` |
| `RDX` | completed operation `1` |
| `RSI` | trusted Agent ID |
| `RDI` | trusted Task ID |
| `R8` | trusted Agent Image ID |
| `R9` | echoed caller nonce |
| `R10`, `R11` | zero |

The runtime resets the single-core call mailbox and resumes this reply frame
under the same Agent CR3. The immutable Worker program replaces only the common
request header, preserving the four reply payload words for its next call.

## Yield

Yield is accepted only when its `RSI`, `RDI`, `R8`, and `R9` words exactly
match the trusted context and nonce from the immediately preceding
DescribeContext round trip. This does not delegate authorization to register
contents: the runtime compares untrusted words against the scheduler-owned
context before exposing `YieldedAgentCpu` evidence.

The architecture boundary then permits the existing semantic adapter to call
`sys_yield_task` for that bound Agent and Task. A valid execution therefore has
two physical Agent calls, four kernel/Agent CR3 transitions, one read-only
reply, and one replayable `TaskYielded` event.

## Assembly And Frame Ownership

The interrupt stub remains a minimal top half: save all integer registers,
switch to kernel CR3, record CR3/RSP/RIP/count evidence, and restore the host
continuation. It no longer labels every call as a yield. Rust decodes the
operation only after selector, address, flags, stack bounds, canary, CR3, and
mailbox evidence are valid.

The first call frame is copied into `SavedAgentFrame`, rewritten with the
DescribeContext reply, and resumed through the existing inverse register-pop
and `iretq` path. The shared RSP0 stack is not reused until the frame is owned.
The second call frame is copied and validated as Yield before semantic state
changes.

## Boot Proof Images

Both verified native Capsules keep the signal wait loop and then execute
DescribeContext followed by Yield. Worker A uses a 72-byte program with call
return offsets 46 and 70. Worker B retains a two-NOP prefix, producing a
74-byte program and offsets 48 and 72. They use different nonzero nonces and
therefore receive and echo different replies under distinct CR3 roots.

The changed payload bytes produce new independently recorded SHA-256 image
digests. Existing Capsule parsing, digest binding, physical readback, and
load-before-dispatch rules remain unchanged.

## Event And Marker Policy

DescribeContext reads trusted execution metadata and mutates no kernel domain
store, so it intentionally emits no semantic event. Yield remains events 43
and 45, preserving the complete 55-event trace and Driver terminal sequence.

QEMU adds separate markers for ABI decoding and successful call return. The
existing Agent-call yield, CR3 switch, heterogeneous execution, and isolation
markers remain required.

## Failure Model

Any malformed request, reply-frame mismatch, zero nonce, context echo mismatch,
unexpected call count, wrong return offset, changed CR3, invalid privilege
frame, dirty mailbox, stack-canary failure, or semantic scheduler mismatch
terminates boot before the corresponding success marker. Invalid calls never
produce task lifecycle events.

## Validation

Host contracts cover exact constants, both request shapes, every common header
failure, operation-specific payload rejection, reply encoding, and context
matching. QEMU must prove two calls per Worker, four address-space transitions,
different nonces and return offsets, all existing isolation/load markers,
exactly 55 semantic events, and the expected `isa-debug-exit` status.

Release disassembly must still show full register capture, kernel CR3
restoration in the call stub, and `iretq` frame restoration for both reply and
normal context resume.

## Implementation Evidence

Development began from two observed failures:

- the focused host contract could not resolve the absent `agent_call` module,
- the unchanged QEMU image completed its prior flow but failed because the new
  ABI marker was missing.

The final implementation passes:

- eight focused ABI contracts covering constants, both requests, all common
  decode failures, operation payloads, reply encoding, zero-ID rejection, and
  context matching,
- every workspace test, including existing image, memory, privilege, and dual
  context contracts,
- the complete 78-event host supervisor scenario,
- no_std library checks and the bare `x86_64-unknown-none` target check,
- host/all-target and bare-target Clippy for `agent-kernel-x86_64` with
  dependencies excluded and warnings denied,
- debug and release QEMU with every marker, exactly 55 ordered semantic events,
  and expected release host exit status 33.

Repository-wide Clippy with dependencies included remains blocked by eight
pre-existing `too_many_arguments` findings in unchanged core modules. This is a
recorded baseline debt rather than a call-ABI regression.

Release symbols place `agent_kernel_enter_user` at `0x10003`,
`agent_kernel_resume_interrupted_user` at `0x1005a`, the timer stub at
`0x1008e`, and the Agent-call stub at `0x10101`. Disassembly shows initial and
reply resume selecting Agent CR3 before `iretq`. The call stub saves all 15
integer registers, restores kernel CR3, records only CR3/RSP/RIP/count/seen
evidence, and returns to the supervisor without classifying the operation.

## Non-goals

V0 does not add arbitrary call registration, capability transfer through
registers, pointer arguments, copy-in/copy-out, shared writable buffers,
asynchronous completion, error replies returned to untrusted code, task
completion, messaging calls, checkpoint calls, dynamic images, multiple CPUs,
or legacy syscall compatibility. These build on the validated envelope and
round-trip boundary.
