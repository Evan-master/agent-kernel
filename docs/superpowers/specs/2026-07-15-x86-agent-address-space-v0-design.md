# X86 Agent Address Space V0 Design

## Status

Accepted for implementation on 2026-07-15.

## Purpose

The privilege-boundary milestone executes one Agent at CPL3, but its pages are
still installed in the kernel's active level-4 page table. This milestone gives
the Agent a distinct CR3 root and proves that every dispatch, preemption, and
Agent call crosses the address-space boundary deliberately.

This belongs to the `agent-kernel-x86_64` architecture layer. It changes x86
page-table ownership and raw CPU transitions; it does not add a POSIX process
model or move semantic policy out of `agent-kernel-core`.

## Architectural Contract

At boot the architecture runtime records the current kernel CR3 and allocates a
fresh, page-aligned P4 frame for the Agent. The Agent P4 inherits every live
kernel P4 entry except the dedicated Agent slot. Inherited entries must be
supervisor-only. The lower page tables behind those entries remain shared so
interrupt text, the IDT/GDT/TSS, RSP0 stack, kernel stack, static evidence, and
the physical-memory window resolve identically in both address spaces.

The fixed Agent region starts at `0x0000_4000_0000_0000`, wholly inside P4
index 128. The kernel P4 entry at index 128 must be unused before and after
preparation. Only the Agent P4 receives the code, signal, guard, and stack
mappings in that slot:

| Region | Agent CR3 | Kernel CR3 |
| --- | --- | --- |
| code page | user, read-only, executable | unmapped |
| signal page | user, read-only, NX | unmapped |
| guard page | unmapped | unmapped |
| four stack pages | user, writable, NX | unmapped |
| inherited kernel mappings | supervisor-only | supervisor-only |

The kernel modifies the signal frame only through its supervisor physical
window. It never needs an alias of the Agent virtual region in the kernel CR3.

## Transition Protocol

The initial dispatch builds a complete CPL3 `iretq` frame while the kernel CR3
is active, switches to the Agent CR3, clears all general-purpose registers, and
then executes `iretq`. Clearing registers prevents kernel pointers and physical
root addresses used by the calling convention from becoming Agent-visible.

On PIT or Agent-call entry, hardware first switches to the TSS RSP0 stack while
the Agent CR3 is still active. The assembly stub then:

1. saves all fifteen general-purpose registers,
2. records the interrupted CR3 in a register,
3. loads the known kernel CR3,
4. records the frame and interrupted-CR3 evidence,
5. returns to the saved kernel continuation with interrupts disabled.

Resume saves a fresh kernel continuation, selects the validated Agent frame,
loads the Agent CR3, restores all registers, and executes `iretq`. The shared
supervisor mappings make the instruction stream and RSP0 frame valid on both
sides of each CR3 write. Rust is never called while the Agent CR3 is active.

## Validation

Host tests lock the dedicated P4 index and the raw CR3 root contract. Bare-metal
validation must prove:

- kernel and Agent P4 frames are distinct and page-aligned,
- inherited P4 entries are identical and supervisor-only,
- the kernel CR3 cannot translate any Agent-owned virtual page,
- the Agent CR3 has the exact least-authority page flags,
- PIT and Agent-call stubs each observed the Agent CR3,
- control returned under the kernel CR3 after both transitions,
- the existing complete privilege frames and RSP0 canary remain valid.

QEMU publishes `AGENT_KERNEL_AGENT_ADDRESS_SPACE_OK` after page-table isolation
is proven and `AGENT_KERNEL_AGENT_CR3_SWITCH_OK` after both round trips are
proven. The semantic event trace remains exactly 40 events because address-space
selection is an implementation consequence of already-recorded dispatch,
preemption, and yield transitions, not a new semantic mutation.

## Failure Model

Preparation fails closed on a missing physical-memory window, an occupied Agent
P4 slot, allocation failure, a user-accessible inherited P4 entry, mismatched
translations, or non-distinct roots. Dispatch and resume fail closed if the
kernel CR3 is not active. Post-transition validation fails on any observed CR3
that differs from the prepared pair.

## Non-goals

V0 does not add multiple runnable Agents, PCID allocation, page-table teardown,
copy-on-write, demand paging, ASLR, arbitrary image loading, SMP TLB shootdown,
SMEP/SMAP, or separate kernel page tables per CPU. The single Agent root is a
real isolation boundary and is intentionally shaped for later fixed-capacity
multi-Agent ownership.

## Dependencies

No new dependency is required. The implementation uses `x86_64` 0.15.5 page
tables and CR3 register semantics already pinned by the architecture crate, plus
the bootloader-provided physical-memory window and Usable frame map.

## Implementation Evidence

The optimized x86_64 ELF produced on 2026-07-15 contains these transition
symbols:

- `agent_kernel_enter_user` at `0xec52`,
- `agent_kernel_resume_interrupted_user` at `0xeca9`,
- `agent_kernel_agent_timer_irq_stub` at `0xecdd`,
- `agent_kernel_agent_call_stub` at `0xed50`,
- `agent_kernel_load_privilege_tables` at `0xf0c1`.

Release disassembly shows the initial `mov cr3, r9` sequence, all fifteen GPR
clears, and `iretq`; resume writes `rdx` to CR3 before restoring the complete
frame. Both interrupt stubs read CR3, load the kernel root from permanent
storage, and only then publish evidence or restore the host continuation.

Debug and release QEMU boots emitted both address-space proof markers, exactly
40 semantic events, and `SUPERVISOR_HANDOFF_READY`. The release image exited
through `isa-debug-exit` with status 33. Workspace tests, the 78-event host
supervisor flow, no_std checks, host and bare-metal Clippy with warnings denied,
formatting, and forbidden-API scans also passed.
