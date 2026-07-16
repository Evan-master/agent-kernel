# X86 Agent General Protection V0 Design

## Status

Implementation in progress on 2026-07-16.

## Purpose

The native x86 runtime already contains a ring-3 invalid opcode, records one
semantic execution fault, and restarts the task from an immutable Capsule
entry. This milestone generalizes that path to a second architecturally
different exception: a ring-3 privileged `cli` instruction raises #GP with a
CPU-pushed error code.

The proof Fault Worker executes `#UD -> recover -> #GP -> recover -> complete`
inside one task, image, capability, and address space. Both faults remain
immutable kernel records. Neither saved exception frame is resumable.

## Layer Placement

- `agent-kernel-core` remains the owner of fault records, rollback authority,
  task state, run ticks, and event order. Its APIs and stores do not change.
- `agent-kernel` remains the only semantic mutation facade through
  `sys_fault_task`, `sys_recover_faulted_task`, and `sys_enqueue_task`.
- The host-testable x86 library owns exception classification, error-code
  encoding, and exact privilege-frame layouts.
- The bare-metal x86 adapter owns IDT replacement, assembly capture, CR3
  restoration, stack validation, physical restart, and boot evidence.

## Boundary Contract

`NativeAgentFault` gains `GeneralProtection { error_code: u32 }`. Boundary
evidence carries the raw 64-bit CPU error-code slot in addition to the vector.
Classification accepts only one internally consistent boundary:

- vector 6 is `InvalidOpcode` and requires error code zero;
- vector 13 is `GeneralProtection` and requires its upper 32 bits to be zero;
- calls and timer expiries require vector and error code zero;
- mixed, repeated, partially set, or unsupported evidence fails closed.

Semantic fault detail is deterministic and backwards compatible:

```text
detail = vector | (error_code << 8)
```

The existing #UD record therefore remains detail 6. The proof `cli` fault has
error code zero and detail 13, while future nonzero #GP selector errors retain
their CPU-provided value.

## Error-Code Stack Frame

Long-mode #GP pushes an error-code slot before RIP, CS, RFLAGS, user RSP, and
user SS. After the assembly stub saves fifteen integer registers, the bounded
RSP0 frame is 168 bytes:

- integer registers: offsets 0 through 112;
- error code: offset 120;
- RIP: offset 128;
- CS: offset 136;
- RFLAGS: offset 144;
- user RSP: offset 152;
- user SS: offset 160.

`PrivilegeErrorCodeStackFrame` models that layout. Capture validates the raw
error code against the classified fault and then normalizes the non-resumable
record into `SavedAgentFrame` by stripping only the error-code slot. Ordinary
interrupt, Agent Call, PIT, and #UD frame layouts remain unchanged.

## Exception Gates And Assembly

The runtime replaces IDT vectors 6 and 13 while IF is clear. Each stub checks
the saved CS before treating an exception as an Agent fault. A CPL3 #GP stub:

1. preserves all integer registers;
2. records the 168-byte RSP0 frame, RIP, Agent CR3, vector 13, and raw error;
3. switches back to the kernel CR3;
4. returns to the saved host dispatch context with IF clear.

A CPL0 #GP branches to the existing fatal vector-13 handler with the original
CPU error-code slot intact. Exceptions do not acknowledge the PIC.

## Restart Generations

The read-only signal ABI keeps byte 2 as the kernel-authored restart
generation. V0 now permits generations 0, 1, and 2. Each consuming restart:

- requires the active kernel CR3 and the exact current generation;
- clears the entire signal page and every writable stack page;
- verifies every cleared byte;
- increments the generation exactly once;
- revalidates the retained page-table roots;
- creates a fresh prepared context at the Capsule entry.

Generation 2 is the hard V0 limit. A third restart fails closed. Physical
preparation does not recover semantic task state; rollback capability is still
required through the public syscall.

## Fault Worker Capsule

After observing physical quantum generation 1, the immutable Capsule branches
on restart generation:

1. generation 0 executes fixed-offset `ud2`;
2. generation 1 executes fixed-offset privileged `cli`;
3. generation 2 issues authenticated `DescribeContext` and `CompleteTask`.

Every generation first expires a real PIT quantum. Both fault paths have no
Agent Call progress, while the final path must authenticate the original
Agent, task, image, capability, and nonce.

## Boot Proof

The existing Worker and Verifier phases remain unchanged through event 91.
The expected extension is:

- events 92-96: recover and re-admit the #UD context;
- event 97: contain #GP(error=0);
- events 98-102: recover and re-admit the #GP context;
- event 103: complete the Fault Worker;
- events 104-113: unchanged driver proof.

Terminal native evidence requires fifteen dispatches, six prepared contexts,
seven preempted contexts, seven physical quantum expiries, two Agent faults,
four completed contexts, no faulted physical contexts, two retained semantic
fault records, and empty physical and semantic queues. The exact boot trace is
113 events.

The fixed-capacity event and fault stores increase the Debug by-value boot
footprint beyond the previous 512 KiB guarded stack. The bootloader contract
therefore reserves a fixed 1 MiB kernel stack while retaining the separate
32 KiB TSS RSP0 stack and its canary checks.

## Failure Policy

The boot proof stops on an incorrect vector, truncated or dirty frame, wrong
error code, wrong CR3, unexpected restart generation, failed page clearing,
missing rollback authority, stale semantic state, reordered fault records,
incorrect Capsule offset, accidental call progress, third restart, or any
attempt to classify a kernel-origin exception as an Agent fault.

## Validation

- Host tests lock classification, detail encoding, rejection cases, and the
  168-byte error-code frame layout and normalization.
- Full workspace tests, Supervisor output, no_std checks, formatting, and
  warnings-denied Clippy remain green.
- Debug and release QEMU require the exact 113-event sequence plus a dedicated
  #GP marker.
- Release disassembly must show CPL-aware vector-6 and vector-13 paths, correct
  #GP offsets, kernel CR3 restoration, fresh entry for both restarts, and no
  restart through either saved exception frame.

## Non-Goals

V0 does not implement page-fault containment, divide-error containment,
automatic crash-loop policy, more than two restarts, checkpoint-data restore,
replacement page allocation, page-table reclamation, SMP migration, nested
fault recovery, or persistent recovery across reboot.
