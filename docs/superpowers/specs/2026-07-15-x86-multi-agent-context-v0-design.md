# X86 Multi-Agent Context V0 Design

## Status

Implemented and verified locally on 2026-07-15. Publication is pending repair
of the configured GitHub CLI credential.

## Purpose

The current x86 path has a real CPL3 Agent address space, but the suspended CPU
frame remains in the single TSS RSP0 stack and global transition mailboxes are
initialized only once. That permits one Agent proof, not multiple simultaneously
suspended Agent contexts.

This milestone proves two admitted Worker Agents with independent CR3 roots,
private physical content, owned suspended frames, and deterministic scheduler
rotation. It belongs to the `agent-kernel-x86_64` architecture and boot adapter
layers. Semantic task authority continues to flow only through public kernel
syscalls.

## Memory Identity

Workers A and B intentionally use the same fixed virtual code, signal, guard,
and stack addresses. Each preparation consumes fresh BootInfo `Usable` frames
for its P4 root, code, signal, and four stack pages. A host-tested
`AgentMemoryIdentity` records those seven physical frames, rejects duplicates,
and proves that both identities are pairwise disjoint. The monotonic boot frame
allocator also keeps all intermediate page-table frames exclusive.

Both roots inherit the same supervisor-only kernel P4 entries. Their dedicated
P4 index 128 points to different lower tables and content frames. The kernel CR3
continues to map neither Agent region. Releasing A's signal through its physical
alias must leave B's signal byte zero.

## Owned CPU Frames

Hardware always builds an interrupt frame on the one TSS RSP0 stack. Normal
kernel context validates that complete frame, then copies all 160 bytes into a
`SavedAgentFrame` owned by the preempted Agent type state. No suspended context
retains an RSP0 pointer. The stack can therefore receive a later interrupt from
the other Agent without corrupting the first context.

Resume sets RSP to the owned frame while still at CPL0, selects that context's
Agent CR3, restores all fifteen GPRs, and executes `iretq`. The owned frame lies
in supervisor memory shared by both roots, while hardware uses TSS RSP0 again on
the next privilege transition.

## Runtime Boundary

One `AgentCpuRuntime` installs the DPL3 call gate and binds the permanent kernel
CR3/RSP0 boundary. It can prepare two context objects only when both roots share
that kernel CR3. Before every initial dispatch or resume, `begin_dispatch`
requires CPL0, IF clear, and the kernel CR3, then resets all per-dispatch
mailboxes. A returned context no longer depends on those mailboxes after its
frame has been copied.

The runtime remains single-core and permits one physically active Agent at a
time. Multiple suspended contexts are real; parallel execution is not claimed.

## Semantic Schedule

The boot adapter registers Worker A as Agent 3 and Worker B as Agent 4. It
creates two delegated tasks that share one verified Worker image, enqueues A
then B, and dispatches A with quantum one. The physical and semantic sequence is:

1. A runs at CPL3 and PIT preempts it; event 37 expires A and queues `[B, A]`.
2. Event 38 dispatches B; B runs and PIT preempts it.
3. Event 39 expires B and queues `[A, B]`; event 40 dispatches A.
4. Only A's signal is released; A resumes from its owned frame and yields at
   event 41, producing queue `[B, A]`.
5. Event 42 dispatches B. B's still-zero signal proves physical isolation, then
   the kernel releases it; B resumes and yields at event 43, producing `[A, B]`.
6. Existing UART/Driver events follow at 44 through 53.

Every mutation remains represented by the existing registration, delegation,
dispatch, quantum-expiry, and yield events. Frame copies and CR3 selection are
implementation consequences of those semantic transitions and add no hidden
kernel-domain mutation.

## Validation

Host tests lock physical identity disjointness and by-value saved-frame
semantics. Bare-metal validation requires:

- two distinct Agent roots with the same kernel root and virtual layout,
- pairwise-disjoint root/code/signal/stack frames,
- A's released signal leaving B's signal clear,
- two independently validated PIT frames surviving RSP0 reuse,
- each resume observing its own Agent CR3 and returning under kernel CR3,
- scheduler queue and execution-context state matching every physical switch,
- exactly 53 semantic events and the existing Driver terminal state.

QEMU publishes `AGENT_KERNEL_MULTI_AGENT_MEMORY_OK`,
`AGENT_KERNEL_AGENT_B_PREEMPTION_OK`,
`AGENT_KERNEL_MULTI_AGENT_CONTEXT_SWITCH_OK`, and
`AGENT_KERNEL_MULTI_AGENT_ISOLATION_OK` only after the corresponding evidence is
validated.

## Implementation Evidence

TDD started with an unresolved `AgentMemoryIdentity`/`SavedAgentFrame` host
contract and a QEMU script requiring the absent multi-Agent marker. The first
dual-context boot then reached `AGENT_KERNEL_MULTI_AGENT_MEMORY_OK` but triple
faulted: QEMU exception state showed the enlarged fixed-capacity `BootedKernel`
crossing the 256 KiB boot stack guard. Raising only the boot kernel stack to 512
KiB restored the guard margin; the separate 32 KiB TSS RSP0 stack did not need
to change.

Both debug and release BIOS images subsequently completed the full A/B sequence,
published all four new proof markers, emitted exactly events 1 through 53, and
exited through `isa-debug-exit` with host status 33. The release ELF exposes:

- `agent_kernel_enter_user` at `0x13e21`, including Agent `mov cr3` and `iretq`;
- `agent_kernel_resume_interrupted_user` at `0x13e78`, including owned-frame RSP,
  Agent `mov cr3`, fifteen GPR pops, and `iretq`;
- `agent_kernel_agent_timer_irq_stub` at `0x13eac`, recording interrupted CR3
  before restoring the kernel CR3;
- `agent_kernel_agent_call_stub` at `0x13f1f`, performing the same CR3 boundary
  on cooperative yield.

Verification passed for the focused architecture contracts, the complete
workspace test suite, the supervisor flow, all no_std library checks, host and
bare-metal Clippy with warnings denied, formatting, diff checks, debug QEMU,
release QEMU, and release disassembly inspection.

## Failure Model

Preparation fails closed on overlapping identities, mismatched kernel roots,
non-kernel CR3, or dirty signal state. Dispatch fails if transition mailboxes
cannot be reset safely. Frame capture fails on any RSP0 bound, selector, flag,
address, CR3, canary, or initial-register mismatch. Semantic progression stops
on any unexpected event, queue order, task status, or execution-context state.

## Non-goals

V0 fixes the capacity at two proof Workers. It does not add a general context
store, dynamic Agent creation, arbitrary image loading, more than one CPU,
parallel execution, PCIDs, TLB shootdown, address-space teardown, demand paging,
or context migration. Those require later ownership and lifecycle milestones.

## Dependencies

No dependency is added. The design uses the existing fixed-capacity kernel,
x86_64 page-table/register support, boot memory map, PIT, TSS RSP0 stack, and
Agent call gate.
