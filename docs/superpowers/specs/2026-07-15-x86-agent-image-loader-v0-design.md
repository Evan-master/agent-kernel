# X86 Agent Image Loader V0 Design

## Status

Implemented and verified locally on 2026-07-15. Publication remains pending
because the configured GitHub CLI credential is invalid.

## Purpose

The x86 runtime now rotates two isolated CPL3 Agent contexts, but both code
pages are filled from one kernel-compiled 21-byte proof program. Semantic
`AgentImageRecord` verification is therefore not yet connected to the bytes the
CPU executes.

This milestone introduces the first native Agent Image Capsule and binds a
verified kernel image identity to one private executable page. Two Worker
Agents load different capsules, enter their declared offsets, and yield from
different code locations under distinct CR3 roots.

## Native Capsule Format

V0 uses a compact AgentOS-native format rather than ELF or a POSIX executable.
All integer fields are little-endian. The fixed 32-byte header is:

| Offset | Bytes | Field | V0 rule |
| --- | ---: | --- | --- |
| 0 | 8 | magic | `AGNTIMG\0` |
| 8 | 2 | format version | `1` |
| 10 | 2 | architecture | `1` = x86_64 |
| 12 | 2 | image kind | `1` = Worker |
| 14 | 2 | flags | `0` |
| 16 | 2 | ABI version | nonzero; equals kernel record |
| 18 | 2 | entry version | nonzero; equals kernel record |
| 20 | 4 | entry offset | strictly inside code |
| 24 | 4 | code length | `1..=4096` |
| 28 | 4 | reserved | `0` |

Exactly `code length` bytes follow the header. Trailing bytes, truncated code,
unsupported fields, and entry offsets outside code fail closed. Only code is
mapped; the capsule header is loader metadata and remains supervisor-owned.

## Integrity And Verification Binding

`sha2` 0.11.0 is added with default features disabled. It is a pure Rust
`#![no_std]` implementation; disabling defaults avoids its optional allocation
feature. The loader computes SHA-256 across the exact header and code bytes and
compares it with `AgentImageRecord.digest`.

The `x86_64-unknown-none` target forces the library's documented compact
software backend through target-specific Cargo rustflags. This avoids runtime
CPU-feature dispatch and x86 SHA intrinsics in the freestanding kernel while
keeping host builds on the library defaults.

A capsule becomes `VerifiedAgentImage` only when:

- its structure and x86_64/Worker format are valid,
- the kernel record status is `Verified`,
- record kind is `Worker`,
- ABI and entry versions equal the capsule header,
- computed and recorded digests are identical.

The type owns no allocation and borrows the immutable capsule bytes. Mutating
one payload or header byte after the expected digest was declared causes
`DigestMismatch`. Core verification remains the authority lifecycle gate; the
x86 loader supplies byte-level evidence for that already-recorded decision.

## Physical Load Boundary

`PreparedAgentMemory::prepare` accepts only a `VerifiedAgentImage`. It allocates
a fresh code frame, zeroes the page, copies exactly the code payload, verifies a
byte-for-byte readback through the supervisor physical alias, and maps the page
read-only and executable. The signal and four stack pages retain their existing
permissions and isolation.

The prepared context stores `code_start + entry_offset`; initial CPL3 entry uses
that address instead of assuming the first byte. Physical copying and page-table
construction are consequences of an existing verified launch and do not mutate
semantic kernel stores.

## Admission And Dispatch Ordering

The boot adapter creates two delegated tasks and two distinct Worker image
records. Each expected digest is compiled independently from its immutable boot
capsule, then registered, verified, and bound to exactly one Worker launch.

Setup stops with both tasks queued. Before the first scheduler dispatch, the
adapter resolves both verified records, validates both capsules, prepares their
private address spaces, and installs CPU contexts. Only then may event 38 move
Worker A to `Running`.

The full sequence is:

1. Existing boot and Driver setup remain events 1 through 15.
2. Worker/task delegation remains events 16 through 27.
3. A registration/verification are events 28 and 29.
4. B registration/verification are events 30 and 31.
5. A and B launch/accept/enqueue are events 32 through 37.
6. A dispatches at event 38; expiries and redispatch are events 39 through 42.
7. A yields at event 43; B dispatches at 44 and yields at 45.
8. Existing UART/Driver flow remains ordered at events 46 through 55.

Worker A uses the original polling program. Worker B has a two-byte NOP prefix,
so its Agent-call return offset differs while behavior remains equivalent. The
runtime validates each call frame lies in that context's executable page and
returns the observed offset to the boot proof. Exact offsets 19 and 21 prove
that both different payloads executed.

## Failure Model

Parsing and digest verification use explicit errors and never map memory.
Loading fails before dispatch on invalid metadata, non-verified status, digest
mismatch, oversized or malformed code, allocation failure, copy/readback
mismatch, or mapping validation failure. CPU entry still fails on privilege,
CR3, selector, stack, flag, mailbox, canary, or frame validation errors.

## Validation

Host contracts cover every header field, exact-length parsing, page-size bounds,
entry bounds, status/kind/version binding, digest mismatch after mutation, and
two distinct capsule digests. QEMU must publish loader and heterogeneous
execution markers, preserve all isolation/context markers, emit exactly 55
semantic events, and exit through `isa-debug-exit` with host status 33.

Release disassembly must continue to show Agent CR3 selection and `iretq` on
initial entry/resume, plus kernel CR3 restoration on timer and Agent-call entry.

## Implementation Evidence

The implementation began from observable failures:

- the host contract could not resolve the absent `agent_image` module,
- QEMU lacked the required image-format marker,
- the first bare SHA-256 build exposed an unsupported automatic x86 backend;
  restricting only `x86_64-unknown-none` to the documented compact software
  backend fixed code generation without changing host builds.

The final verification matrix passed:

- seven focused image-loader contracts, including the standard SHA-256 `abc`
  vector, plus existing user-memory and dual-context contracts,
- every workspace test and the 78-event host supervisor scenario,
- the no_std library and bare x86 target checks,
- host and bare-target Clippy with warnings denied,
- debug QEMU with all markers and exactly 55 ordered semantic events,
- release QEMU with the same evidence and expected host exit status 33.

Release symbol inspection located `agent_kernel_enter_user` at `0x15610`,
`agent_kernel_resume_interrupted_user` at `0x15667`, the timer stub at
`0x1569b`, and the Agent-call stub at `0x1570e`. Disassembly shows initial
entry selecting the supplied CR3 before `iretq`, resume restoring the saved
frame before `iretq`, and both interrupt gates restoring kernel CR3 before
returning to the supervisor context.

## Non-goals

V0 does not add ELF, dynamic linking, relocations, writable data segments,
multiple code pages, signatures, trust roots, compression, package discovery,
filesystem loading, demand paging, ASLR, page-table teardown, dynamic context
capacity, or SMP. Those build on this verified one-page load boundary.
