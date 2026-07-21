# Right-sized Agent Code Ownership V7 Design

Status: Implemented and validated on 2026-07-21

## Objective

Replace the fixed four-code-frame physical profile with bounded per-image
ownership. Capsule v1 keeps its four-page executable limit and stable virtual
layout. Each Agent owns only the physical code frames required by its declared
code length.

V7 establishes a variable-size identity and reclamation protocol before the
executable window grows beyond four pages. The protocol remains allocation-free,
deterministic, and suitable for later demand-backed executable memory.

## Stable Contracts

- Capsule v1 keeps the 32-byte header and `1..16,384` code-length range.
- `AGENT_REGION_BASE` and every ring-3 virtual address remain unchanged.
- Code pages remain user-readable and executable, with writable and NX bits
  clear.
- Signal, stack, lazy-data, runtime-region, and call-data ownership remain one
  fixed seven-frame set.
- Every Agent keeps four private page-table frames.
- SHA-256 continues to bind the complete Capsule bytes to the kernel image
  record.

## Variable Physical Identity

For `code_page_count` in `1..=4`:

```text
content_frame_count = code_page_count + 7
owned_frame_count   = 4 + content_frame_count
```

| Capsule code | Code pages | Content frames | Owned frames |
| :--- | ---: | ---: | ---: |
| `1..4,096` bytes | 1 | 8 | 12 |
| `4,097..8,192` bytes | 2 | 9 | 13 |
| `8,193..12,288` bytes | 3 | 10 | 14 |
| `12,289..16,384` bytes | 4 | 11 | 15 |

`AgentMemoryIdentity` stores bounded arrays plus the active code-page count.
Only the active prefix participates in alias checks, ownership transfer,
zeroing, and equality evidence. Inactive storage must be canonical zero. A
physical frame at address zero remains valid when it belongs to the active
prefix.

## Loading And Mapping

Boot preparation allocates `VerifiedAgentImage::code_page_count()` code frames.
Reused preparation requests the same count from the reclaimed frame pool. The
loader zeros, copies, and reads back only those frames.

Page-table installation maps the active code prefix. Validation proves every
active page is RX and every unused page in the four-page code window is
unmapped. The kernel root continues to exclude the complete Agent region.

Restart retains the active immutable code prefix. Final teardown clears every
active content frame and all four private page-table frames.

## Transactional Reclamation

The fixed-capacity pool stores raw physical frames and accepts variable-size
identities. Allocation takes the exact suffix required for the requested code
page count and reconstructs a fresh identity from those frames. Generation
tokens still reject stale commits. Cancellation returns the complete owner in
its original frame order.

The first six boot Agent address spaces introduce the pool's physical frame
inventory. The kernel seals that inventory after all six have terminated and
their frames are zero. Later admissions may only transfer frames inside the
sealed count. `all_reclaimed_and_zero()` requires the sealed inventory to be
fully present and byte-zero.

## Native Evidence

The boot proof uses heterogeneous Capsule sizes:

- Worker, Verifier, fault, handler, reuse, and Admission Supervisor images use
  one code frame each;
- the Resource Manager crosses the first 4 KiB boundary and uses two code
  frames;
- pool length deltas are derived from each admitted identity;
- two runtime-admission batches reclaim, resize, reuse, cancel, and restore the
  sealed inventory without exposing resident frames.

The runtime emits `AGENT_KERNEL_NATIVE_RIGHT_SIZED_CODE_FRAMES_OK` after the
heterogeneous initial identities and sealed inventory agree.

Measured native profile:

| Evidence | Value |
| :--- | ---: |
| Initial Agent address spaces | 6 |
| One-page identities | 5 x 12 frames |
| Two-page identities | 1 x 13 frames |
| Sealed boot inventory | 73 frames |
| Previous fixed inventory | 90 frames |
| Frames removed from the boot profile | 17 |

Debug and Release QEMU completed Events `1..409`, restored all 73 frames,
replayed the Event archive, and reached `SUPERVISOR_HANDOFF_READY`.

## Verification Gates

- identity constructors reject zero-page, over-capacity, aliased, unaligned,
  and noncanonical inactive-frame inputs;
- host contracts prove 12, 13, 14, and 15-frame identities;
- pool contracts prove variable-size allocation, stale-token rejection,
  cancellation, frame-zero ownership, and exact restoration;
- mapping validation proves unused code-window pages stay unmapped;
- focused tests fail before implementation and pass afterward;
- Workspace tests, Supervisor simulation, `no_std`, strict Clippy, debug and
  Release QEMU, and Release ELF audits pass.

Final gates:

| Gate | Evidence |
| :--- | :--- |
| Workspace | `216` result groups, `745` passed tests |
| Supervisor | Host simulation completed through Event archive checkpoint |
| Freestanding | Five `no_std` libraries plus the bare-metal binary passed |
| Lints | Workspace and bare-metal Clippy passed with warnings denied |
| QEMU | Debug and Release produced the exact Events `1..409` transcript |
| Ownership | Every admission delta matched its identity's exact frame count |

## Deferred Work

- a larger virtual executable window and Capsule size bound;
- segmented code, read-only data, and relocation records;
- signed package manifests and measured storage-backed loading;
- demand-paged executable frames;
- SMP instruction-TLB synchronization.
