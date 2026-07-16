# X86 Agent Page Fault Repair V1 Design

## Status

Implemented and validated locally on 2026-07-16; publication is pending.

## Purpose

Page Fault V0 contains protection violations and restarts the affected Agent at
its immutable entry. V1 proves a different operating-system primitive: one
bounded not-present data page can be activated under explicit recovery
authority, after which the original faulting frame resumes at the same RIP and
the instruction succeeds without restarting the Capsule.

The ordered Fault Worker proof becomes:

```text
#UD -> restart -> #GP -> restart -> protection #PF(7) -> restart
    -> not-present #PF(6) -> map -> resume same frame -> complete
```

## Layer Placement

- `agent-kernel-core` and `agent-kernel` remain unchanged. The physical repair
  is still paired with public `sys_fault_task`, `sys_recover_faulted_task`, and
  `sys_enqueue_task` transitions under explicit Rollback authority.
- The host-testable x86 library owns the extended user layout, seven-frame
  memory identity, and existing deterministic page-fault detail encoding.
- The bare-metal x86 adapter owns the reserved frame, initially absent PTE,
  one-way PTE activation, fault-frame ownership, runtime parking, and exact
  QEMU evidence.

## Lazy Data Page

`UserMemoryLayout` adds one page immediately above the four-page Agent stack:

```text
0x0000_4000_0000_7000  lazy data page
```

Preparation allocates and zeroes a private physical frame for this address but
does not install its PTE. The frame becomes the seventh content frame in
`AgentMemoryIdentity`, so roots and all content frames remain pairwise disjoint
across Agents. The kernel root must not translate the address, and the initial
Agent root must also report it absent.

Every full entry restart clears the reserved frame together with the signal and
stack frames. The frame is never allocated at fault time, which keeps repair
bounded and deterministic.

## Activation Contract

The only supported activation maps the retained private frame at the fixed lazy
address with exactly these leaf properties:

- present;
- user accessible;
- writable;
- non-executable;
- 4 KiB page;
- absent before activation.

Repair requires the kernel CR3 to be active. The existing Agent lower page
tables already cover the adjacent stack, so activation writes only the unused
leaf entry and allocates no table. The mapping is validated by translation
before ownership can leave the memory object. A second activation, a different
address, a different frame, a huge-page path, or a non-kernel caller fails
closed.

Loading the Agent CR3 for repaired-frame resume provides the required non-global
translation invalidation; V1 does not enable PCID.

## Recoverable Fault Contract

Restart generation 3 waits for one real physical quantum and executes a byte
write of `0x5a` to the absent lazy page. The required CPU evidence is:

- vector 14;
- error code 6 (`P=0`, `W/R=1`, `U/S=1`);
- `CR2=0x0000_4000_0000_7000`;
- saved RIP at the Capsule's fixed lazy-write instruction;
- restart generation 3;
- no prior Agent Call progress in this proof generation.

The semantic detail is:

```text
0xe006_4000_0000_7000
```

Protection #PF(7) at the signal page remains non-resumable and follows the V0
restart path. No other address, error code, vector, generation, or instruction
is repairable.

The final Fault Worker Capsule is 148 bytes: a 32-byte header plus 116 bytes of
code. Its ordered fault offsets are 42 (#UD), 44 (#GP), 47 (protection #PF),
and 62 (lazy #PF); its DescribeContext and CompleteTask return offsets are 105
and 114. The SHA-256 digest over the exact Capsule is
`93278aa869234eddd57aa7c716bc3664d8950944d69ed13b4404b421068b836b`.

## Same-Frame Resume

`FaultedAgentCpu` retains the complete `AgentCallProgress` rather than reducing
it to a Boolean. Consuming the exact repairable fault:

1. activates the lazy mapping;
2. keeps the normalized `SavedAgentFrame` RIP unchanged;
3. records bootstrap-authorized `TaskFaultRecovered`;
4. parks the CPU as a distinct `RecoveredFault` runtime state;
5. requeues the task through the public scheduler;
6. rearms a fresh PIT quantum and returns with `iretq` to the original write.

The CPU-pushed page-fault error slot is not resumed. It was already removed
when the 168-byte frame became a 160-byte `SavedAgentFrame`. Linear frame
ownership makes the original fault object impossible to resume or repair twice.

After retry, the Capsule reads back `0x5a` before issuing authenticated
DescribeContext and CompleteTask calls. The terminal physical report also reads
the byte through the supervisor alias, independently proving that the intended
private frame changed.

## Semantic And Event Proof

Events through V0 event 108 remain unchanged. V1 extends the sequence:

- event 109: not-present lazy-page fault;
- event 110: authorized fault recovery;
- event 111: recovered task queued;
- event 112: repaired frame dispatched;
- event 113: Fault Worker completed;
- events 114-123: unchanged Driver proof.

Terminal runtime evidence requires eighteen dispatches, seven prepared
contexts, eight preempted contexts, one repaired-fault context, one mailbox
wait, one yield, eight physical quantum expiries, four Agent faults, four
completed contexts, four immutable semantic fault records, and empty semantic
and physical queues. The task retains four run ticks and restart generation 3.

## Failure Policy

The boot proof fails closed on a wrong error bit, CR2 value, fault RIP, restart
generation, physical frame, PTE state, mapping flag, page size, recovery
authority, task state, event order, data byte, transcript, runtime variant,
counter, or queue state. Kernel-origin page faults remain fatal.

## Validation

- Host red/green tests lock the lazy address, seven-frame identity, #PF(6)
  classification, and semantic detail.
- Formatting, full workspace tests, Supervisor output, no_std checks, and
  warnings-denied scoped Clippy remain green.
- Debug and release QEMU require
  `AGENT_KERNEL_NATIVE_AGENT_DEMAND_PAGE_OK` and exactly 123 events.
- Release disassembly rechecks vector-14 saved-CS selection, CR2 capture,
  error/RIP offsets 120/128, CR3 restoration, and CPL0 fatal fallback.

## Non-Goals

V1 does not provide a general pager, arbitrary virtual allocation, fault-time
frame allocation, multiple lazy pages, eviction, swapping, copy-on-write,
file-backed mappings, stack growth, executable data, unmapping, remapping,
replacement address spaces, user pager upcalls, Agent-selected mapping policy,
PCID, SMP TLB shootdown, or recovery from any fault other than the one exact
not-present proof page.
