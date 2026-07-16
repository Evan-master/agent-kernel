# X86 Agent Page Fault V0 Design

## Status

Implemented, validated, merged, and published on 2026-07-16 in feature commit
`9e77d77`.

## Purpose

The native runtime now contains #UD and #GP faults, but a ring-3 memory
protection violation still reaches the fatal vector-14 baseline. This milestone
contains one Agent page fault, preserves its CPU error code and CR2 address,
recovers the task through ordinary rollback authority, and proves that a fresh
fourth execution generation can complete.

The existing Fault Worker becomes a single ordered proof:
`#UD -> recover -> #GP -> recover -> #PF -> recover -> complete`.

## Layer Placement

- `agent-kernel-core` and `agent-kernel` remain unchanged. Faults still enter
  through public `sys_fault_task`, `sys_recover_faulted_task`, and
  `sys_enqueue_task` operations.
- The host-testable x86 library owns vector-14 classification, canonical user
  address validation, and deterministic semantic detail encoding.
- The bare-metal x86 adapter owns CR2 capture, IDT replacement, frame and
  mailbox validation, physical restart, and exact boot evidence.

## Boundary Contract

`NativeAgentFault` gains:

```text
PageFault { error_code: u16, address: u64 }
```

Boundary evidence adds the fault-address mailbox. Calls, PIT expiries, #UD, and
#GP all require that field to be zero. Page-fault classification requires:

- exactly one fault boundary and vector 14;
- a raw CPU error code no larger than `0x0fff`;
- a lower-half canonical address no larger than `0x0000_7fff_ffff_ffff`;
- no Agent Call, timer, or preemption evidence.

The 12-bit V0 error range includes the architectural present, write, user,
reserved-bit, instruction-fetch, protection-key, shadow-stack, and RMP flags.
SGX bit 15 and upper-half addresses are outside this Agent user-space V0.

## Semantic Detail

#UD detail 6 and #GP `vector | (error_code << 8)` remain unchanged. Page-fault
detail preserves vector, error, and exact lower-half address in one word:

```text
detail = (14 << 60) | (error_code << 48) | address
```

The proof address is the read-only signal page at `0x0000_4000_0000_1000` and
the protection-write-user error code is 7, producing detail
`0xe007_4000_0000_1000`.

## Vector-14 Capture

Long-mode #PF uses the same 168-byte privilege error-code frame as #GP. The
vector-14 entry checks saved CS before containment, saves all integer registers,
reads CR2 and the active Agent CR3, switches to the kernel CR3, and records:

- frame RSP;
- fault RIP;
- vector 14;
- raw CPU error code;
- CR2 fault address;
- count and seen flags.

Capture requires the classified error and address to match the mailboxes,
normalizes the error-code frame into a non-resumable `SavedAgentFrame`, and
validates the original Agent mappings and selectors. A CPL0 #PF keeps its CPU
error slot and jumps to the existing fatal vector-14 handler.

## Signal-Page Protection Proof

Restart generation 2 first expires a real PIT quantum, then executes a byte
write to the read-only signal-page base. Page-table permissions must reject the
write before any signal byte changes. The expected page-fault error is:

- P = 1: the page is present;
- W/R = 1: the access is a write;
- U/S = 1: the access originated in ring 3.

CR2 must equal the signal-page base and saved RIP must equal the Capsule's
fixed write-instruction offset.

## Third Restart

The restart-generation ABI extends from maximum 2 to maximum 3. The same
consuming transition clears the complete signal page and every stack page,
verifies zeros, writes generation 3, and creates a prepared CPU context at the
immutable Capsule entry. A fourth restart remains unsupported.

The third public recovery must retain all three fault records, preserve task
run ticks, clear the semantic execution context to Idle before queueing, and
use the bootstrap capability's explicit Rollback authority.

## Capsule And Event Proof

The immutable Capsule branches after physical quantum generation 1:

1. generation 0 executes `ud2`;
2. generation 1 executes privileged `cli`;
3. generation 2 writes the read-only signal page and raises #PF(7);
4. generation 3 performs authenticated DescribeContext and CompleteTask.

The assembled code is 92 bytes. Its #UD, #GP, and #PF instruction offsets are
42, 44, and 47; Agent Call return offsets are 81 and 90. The exact 124-byte
Capsule digest is
`0b8c9a9c6e4164457943393ae2559fc9bb8680b5c23bc6546fe853c24e6ffa13`.

Events through the #GP recovery remain unchanged. The extension is:

- event 103: page fault;
- events 104-108: third recovery, queue, dispatch, PIT expiry, redispatch;
- event 109: Fault Worker completion;
- events 110-119: unchanged Driver proof.

Terminal evidence requires seventeen dispatches, seven prepared contexts,
eight preempted contexts, eight physical quantum expiries, three Agent faults,
four completed contexts, three immutable semantic fault records, and empty
physical and semantic queues. The exact trace contains 119 events.

## Failure Policy

The proof fails closed on a wrong CR2 value, wrong error bit, noncanonical
address, unsupported error bit, dirty signal byte, incorrect fault offset,
kernel-origin #PF, missing rollback authority, stale semantic state, lost or
reordered fault history, accidental frame resume, generation above 3, or any
event/counter mismatch.

## Validation

- Host tests lock page-fault classification, detail encoding, canonical-address
  rejection, conflicting address evidence, and generation 3.
- Full workspace tests, Supervisor output, no_std checks, formatting, and
  scoped warnings-denied Clippy remain green.
- Debug and release QEMU require a dedicated page-fault marker and exactly 119
  events.
- Release disassembly must show saved-CS CPL selection, CR2 capture, error/RIP
  offsets 120/128, vector 14, kernel CR3 restoration, and the CPL0 fatal path.

## Non-Goals

V0 does not implement demand paging, lazy allocation, copy-on-write, page-fault
upcalls, replacement address spaces, mapping repair, stack growth, swapping,
SGX faults, upper-half Agent mappings, more than three restarts, automatic
retry policy, nested faults, or kernel page-fault recovery.
