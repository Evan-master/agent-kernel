# X86 Native Runtime Quantum V0 Design

## Status

Accepted for autonomous implementation on 2026-07-16.

## Purpose

Native Agent Runtime Loop V0 routes every ABI V1 operation through one generic
execution loop, but physical PIT preemption still applies only to the first
entry of each Capsule. Once a DescribeContext or another returning Agent Call
has replied, the owned frame resumes without reloading a timer. An Agent can
therefore compute forever between two calls while retaining the CPU.

This milestone makes every transition back to ring 3 start a fresh physical
quantum. A resumed Agent may return to the kernel through either the Agent Call
gate or PIT IRQ0. A timer return preserves the complete call session, including
trusted context, nonce, transcript, private memory, and saved frame, then uses
the normal public task tick transition to rotate the FIFO.

The proof extends Worker A with deterministic computation after SendMessage.
Worker B is already runnable at that point, so A's physical quantum expiry must
place A behind B. B then completes before A resumes the same session, issues
Yield, and completes. No role-specific execution path is reintroduced.

## Layer Placement

- `agent-kernel-core` keeps sole ownership of semantic task quantum, FIFO, and
  event transitions. No new core operation is needed.
- `agent-kernel` remains the only mutation facade used by the x86 executor.
- The host-testable `agent-kernel-x86_64` library defines the mutually
  exclusive physical boundary evidence contract.
- The bare-metal x86 adapter arms hardware, captures frames, preserves session
  ownership, updates the read-only quantum generation, and emits boot proof.
- Capsule code only observes a read-only architecture page and never writes
  scheduler state or capability data.

## Physical Boundary Contract

Every return from one ring-3 run is classified from a reset evidence mailbox:

1. `AgentCall` requires exactly one call-gate capture, no timer IRQ capture,
   and no preemption flag.
2. `QuantumExpired` requires exactly one timer capture, no call-gate capture,
   and the preemption flag.
3. Empty, mixed, repeated, or internally inconsistent evidence is rejected.

`NativeRunBoundaryEvidence` is fixed-width, allocation-free, and host tested.
The classifier does not decode calls, inspect frames, or mutate scheduler
state. The bare-metal adapter separately validates the selected frame, CR3,
RSP0 bounds, selectors, RIP, RSP, and kernel continuation.

## Arm And Disarm Discipline

The runtime installs and programs PIT channel 0 before every ring-3 entry or
resume, not only before a prepared Capsule first runs. The evidence mailbox is
reset before arming. Every return path reaches normal context with IF clear and
masks IRQ0 before inspecting the mailbox.

The Agent Call interrupt gate clears IF before its assembly stub switches back
to the saved host stack. The PIT stub masks IRQ0 and acknowledges the PIC before
the same switch. Reprogramming channel 0 and reinitializing the single exposed
legacy IRQ on the next run prevents an old periodic count from becoming the new
quantum.

## Session-Preserving Preemption

The physical call session stores:

- prepared Agent memory and address-space roots;
- the installed CPU runtime boundary;
- one complete owned privilege frame;
- trusted Agent/Task/Image/capability context;
- optional DescribeContext nonce;
- fixed-capacity operation and return-offset transcript.

`PreemptedAgentCpu` carries the same session progress. Its initial admission
instance has no nonce and an empty transcript. A later quantum expiry converts
a live resumable session back into `PreemptedAgentCpu` without clearing either.
Redispatch resumes the captured RIP and registers under the same CR3. The next
call must still authenticate against the original nonce, and transcript append
continues from its previous length.

Running a preempted or resumable CPU returns a typed outcome:

- `Call(PendingAgentCallCpu)` when the call gate won the boundary;
- `Preempted(PreemptedAgentCpu)` when IRQ0 won the boundary.

The inner operation loop handles `Call` immediately. `Preempted` records one
public `TaskQuantumExpired` transition, parks the owned CPU, and returns to the
outer kernel-selected dispatch loop.

## Read-Only Quantum Generation

The existing private signal page is supervisor-writable and Agent-readable but
not Agent-writable or executable. Byte 0 remains the one-time admission release
used before DescribeContext. Byte 1 becomes a wrapping-prohibited physical
quantum generation counter.

After a timer frame is fully validated, the kernel increments byte 1 through
the private supervisor alias. Overflow fails closed. The counter is evidence
and an observation surface; it does not replace the semantic task tick, choose
a runnable Agent, or grant authority.

Worker A starts with generation 0. Its admission PIT expiry changes it to 1.
After SendMessage returns and wakes Worker B, A spins on its read-only generation
until it reaches 2. The newly armed PIT must interrupt that loop, validate the
frame, and increment the generation. A's semantic tick then appends A behind B.
This proves exactly one post-call quantum without relying on wall-clock loop
counts or host performance.

## Scheduler And Event Sequence

The existing first three admission expiries remain unchanged. The new Worker A
expiry adds two semantic events after MessageWaitWoken:

- event 59: A's `TaskQuantumExpired`, producing queue `[B, A]`;
- event 60: kernel-selected dispatch of B's waiting receive context.

B still receives, acknowledges, submits its result, and completes. Event 65
then dispatches A's session-preserving preempted context. A reaches Yield at
event 66, is redispatched from Yielded at event 67, and completes at event 68.
Verifier setup and the Driver proof shift by two events without changing their
relative semantics. The final trace contains exactly 86 events.

The runtime evidence contract becomes nine dispatches: three Prepared, four
Preempted, one WaitingCall, and one YieldedCall. Exactly four physical quantum
expiries occur, and exactly one carries a non-empty call transcript.

## Capsule And Transcript Evidence

Worker A inserts a signal-generation wait after its SendMessage return. Its
first three return offsets stay unchanged; Yield and CompleteTask offsets move
by the inserted instruction length. The Capsule code length and verified digest
change accordingly. Worker B and Verifier Capsules remain byte-for-byte stable.

Worker A's terminal transcript must still contain exactly five operations in
the same order and the same nonce. The additional physical preemption does not
append a call or add an address-space call switch. This distinguishes CPU
quantum evidence from Agent Call evidence.

QEMU adds:

`AGENT_KERNEL_NATIVE_RUNTIME_QUANTUM_OK`

It is emitted only after the executor proves the post-call expiry, FIFO order,
preserved session transcript, generation 2, empty physical registry, and all
existing terminal task states.

## Failure And Atomicity

- PIT setup failure occurs before entering ring 3.
- Invalid or mixed boundary evidence fails before any semantic task tick.
- A timer frame is copied and validated before the quantum generation changes.
- Generation overflow fails closed and never wraps to an earlier observation.
- The semantic tick must succeed before the preempted CPU is parked.
- A call reply is still encoded only after its matching semantic operation.
- A post-call preemption never appends an operation or changes the nonce.
- A physical registry insertion failure retains the rejected CPU ownership
  value and terminates the deterministic boot proof.

## Validation

- A red host test defines exact physical boundary classification and rejection.
- Existing transcript and ABI tests remain green.
- Full workspace tests and the 78-event Supervisor remain unchanged.
- no_std bare-metal check and scoped Clippy pass with warnings denied.
- Debug and release QEMU require the new marker and exact 86-event sequence.
- Release disassembly confirms every entry/resume path still switches CR3,
  saves complete frames, and restores through `iretq`.

## Non-Goals

V0 does not add dynamic timer frequencies, per-task quantum lengths in hardware
ticks, APIC/IOAPIC, SMP timers, nested interrupt handling, preemptible kernel
code, multiple tasks per Agent, context migration, error replies, or real-time
scheduling. It proves one hardware tick per semantic quantum on the existing
single-core legacy PIT path.
