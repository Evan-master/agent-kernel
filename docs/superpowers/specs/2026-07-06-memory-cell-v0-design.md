# Memory Cell V0 Design

## Purpose

Memory Cell V0 adds a native kernel state primitive for agents. It is not a
file, heap allocation, database row, or host-side cache. It is a deterministic
fixed-capacity kernel store for small typed state that agents can remember and
recall through explicit capabilities.

## Scope

V0 provides:

- first-class `MemoryCellId`, `MemoryValue`, and `MemoryCellRecord` types,
- fixed-capacity memory cell storage owned by `KernelCore`,
- `create_memory_cell(agent, capability, resource, value)` for creating a cell
  under a `ResourceKind::Memory` resource,
- `recall_memory_cell(agent, capability, cell)` for auditable reads,
- `remember_memory_cell(agent, capability, cell, value)` for auditable writes,
- replayable `MemoryCellCreated`, `MemoryCellRecalled`, and
  `MemoryCellRemembered` events,
- facade syscalls and read-only memory cell inspection.

V0 intentionally does not provide byte arrays, dynamic allocation, memory maps,
virtual addressing, files, shared memory pages, host persistence, encryption, or
garbage collection.

## Core Model

```rust
pub struct MemoryCellId(u64);

pub struct MemoryValue {
    pub words: [u64; 4],
}

pub struct MemoryCellRecord {
    pub id: MemoryCellId,
    pub resource: ResourceId,
    pub creator: AgentId,
    pub last_writer: AgentId,
    pub value: MemoryValue,
    pub revision: u64,
}
```

`KernelCore` gains an explicit `MEMORY_CELLS` capacity, a fixed memory cell
array, a memory cell length, and a deterministic `next_memory_cell` counter.

## Authority And Ordering

Memory cells are scoped to resources of kind `ResourceKind::Memory`.

Creating and remembering require `Operation::Act` authority on the memory
resource. Recalling requires `Operation::Observe` authority on the cell's memory
resource. All actor checks use the existing active-agent boundary, so unknown,
suspended, and retired actors fail before cell lookup or event mutation.

Every successful memory operation appends exactly one event. Creating starts at
revision `1`. Remembering updates the fixed value, increments revision by `1`,
updates `last_writer`, and records `MemoryCellRemembered`. Recalling returns the
value and records `MemoryCellRecalled`; if the event log is full, the value is
not returned because the recall would be unaudited.

Capacity, authorization, resource-kind, lookup, and event-log failures leave
memory cells and events unchanged.

## Test Evidence

Tests must prove:

- creating records a memory cell and `MemoryCellCreated`,
- recall returns the stored value and records `MemoryCellRecalled`,
- remember updates value, revision, last writer, and records
  `MemoryCellRemembered`,
- non-memory resources return `ResourceKindMismatch` without an event,
- missing cells return `MemoryCellNotFound`,
- missing authority returns `OperationDenied` without mutation,
- suspended actors are rejected before memory cell lookup,
- store-full and event-log-full failures are atomic,
- facade syscalls expose the same behavior through `AgentKernel`.
