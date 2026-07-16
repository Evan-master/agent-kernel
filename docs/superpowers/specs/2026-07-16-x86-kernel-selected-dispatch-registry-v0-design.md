# X86 Kernel-Selected Dispatch Registry V0 Design

## Status

Accepted for autonomous implementation on 2026-07-16.

## Purpose

The task run queue is kernel-owned, but every current dispatch call still asks
the boot adapter to supply the Agent expected at the head. The adapter also
keeps three prepared x86 CPU objects in named local variables and manually
chooses one after each semantic dispatch. This makes the demonstrated schedule
correct but leaves physical context selection outside the kernel result.

This milestone makes dispatch kernel-selected. The core consumes its own FIFO
head and returns the exact Agent/Task identity it made Running. A bounded native
runtime registry then resolves that kernel-selected Agent ID to the exclusively
owned prepared x86 CPU object. Worker and Verifier first dispatches must use this
path; no named CPU variable may select a physical context before the kernel.

## Core Dispatch Contract

The core adds:

```rust
dispatch_next_ready_with_quantum(quantum) -> RunQueueEntry
```

`RunQueueEntry` is reused as the compact `{ agent, task }` dispatch outcome.
The operation rejects zero quantum or an empty queue, reads the FIFO head,
validates that head Agent is active, its execution context is Idle, its task is
Accepted and admitted, and one event slot remains. It then removes the head,
makes the task and execution context Running, records `TaskDispatched`, and
returns the consumed entry.

Existing `dispatch_next_with_quantum(agent, quantum)` remains as a compatibility
wrapper. It first rejects an Agent that does not match the current queue head,
then delegates to the same mutation path. This preserves existing error and
event behavior while ensuring both APIs have one implementation.

The facade exposes `sys_dispatch_next_ready_with_quantum`. No Agent capability
is introduced: scheduler selection remains a trusted kernel operation, while
task admission still validates the launch entry and delegated authority.

## Native Runtime Registry

The architecture library adds a generic fixed-capacity
`NativeAgentRuntimeStore<T, N>`. Each occupied slot owns one non-Copy runtime
value under one nonzero `AgentId`. It supports deterministic insertion and
ownership-taking by Agent ID, rejects duplicate IDs and capacity exhaustion,
and compacts after a take so capacity is immediately reusable.

The store uses only arrays, `Option`, typed IDs, and explicit errors. It has no
heap, host I/O, global state, callbacks, scheduler policy, or CPU instructions.
Host tests use a non-Copy value to prove that taking one Agent transfers exactly
one ownership token and preserves the remaining registration order.

The bare-metal boot adapter specializes the store to `PreparedAgentCpu` with
capacity three. Preparation inserts Worker A, Worker B, and the Verifier only
after Capsule verification, isolated memory creation, and trusted call-context
construction. Registration derives its key from the CPU object's
scheduler-owned context Agent ID, so the boot adapter cannot supply a different
identity.

## Physical Schedule

Semantic events remain exactly unchanged:

1. B and A are queued in that order.
2. Kernel-selected dispatch returns B; the registry transfers B's prepared CPU.
3. B is preempted and expires. Kernel-selected dispatch returns A; the registry
   transfers A's prepared CPU.
4. A is preempted and expires. Kernel-selected redispatch returns B, whose
   already-owned suspended frame continues through blocking mailbox IPC.
5. After both Workers complete, the Verifier is queued. Kernel-selected dispatch
   returns the Verifier and the registry transfers the final prepared CPU.
6. Redispatches of already-suspended contexts continue to use their owned frame
   type states in V0.

The registry is empty after the Verifier's first dispatch. Two new QEMU proof
markers are emitted only after the exact returned identities and store counts
have been checked:

- `AGENT_KERNEL_KERNEL_SELECTED_DISPATCH_OK`
- `AGENT_KERNEL_NATIVE_RUNTIME_STORE_OK`

## Validation

Core tests cover FIFO identity return, state/event mutation, zero quantum,
empty queue, inactive head Agent, busy context, revoked admission, and
compatibility-wrapper mismatch without partial mutation. Facade tests prove
the new syscall does not require the caller to predict an Agent ID.

Architecture host tests cover empty lookup, insertion order, duplicate IDs,
zero IDs, capacity, ownership transfer, compaction, and capacity reuse. Existing
Agent-call tests gain context identity access checks.

Debug and release QEMU must emit both new markers and the existing blocking
mailbox, Verifier, and Driver markers while preserving exactly 82 ordered
events. Full workspace tests, Supervisor output, no_std checks, scoped Clippy,
formatting, and release disassembly inspection remain required.

## Failure Model

Every core validation occurs before queue, task, context, or event mutation.
Registry insertion failure leaves all existing values owned by their slots and
returns the rejected value to the caller. A missing physical context after a
successful semantic dispatch is a fatal boot invariant violation; the adapter
does not substitute another Agent or roll the kernel schedule backward.

## Non-Goals

V0 does not store post-preemption frames, retained Agent-call sessions, Waiting
contexts, or completed contexts in the registry. It does not add runtime Agent
creation, arbitrary Capsule sources, address-space teardown, slot replacement,
priorities, work stealing, SMP dispatch, context migration, or preemptive
scheduler policy beyond the existing FIFO and fixed quantum.
