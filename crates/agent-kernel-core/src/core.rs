//! Fixed-capacity Agent Kernel core state machine.
//!
//! This module owns resource registration, capability grants, authorization,
//! event recording, checkpoint creation, and rollback requests. It performs no
//! host I/O and keeps state deterministic for replay and supervisor inspection.

use crate::{
    ActionRecord, AgentRecord, Capability, CheckpointRecord, Event, Intent, MemoryCellRecord,
    MessageRecord, NamespaceEntryRecord, ObservationRecord, Resource, RunQueueEntry, Task,
};

#[derive(Debug)]
pub struct KernelCore<
    const AGENTS: usize,
    const RESOURCES: usize,
    const CAPS: usize,
    const EVENTS: usize,
    const ACTIONS: usize,
    const OBSERVATIONS: usize,
    const CHECKPOINTS: usize,
    const INTENTS: usize,
    const TASKS: usize,
    const RUN_QUEUE: usize,
    const MESSAGES: usize = 0,
    const MEMORY_CELLS: usize = 0,
    const NAMESPACE_ENTRIES: usize = 0,
> {
    pub(crate) agents: [AgentRecord; AGENTS],
    pub(crate) resources: [Option<Resource>; RESOURCES],
    pub(crate) capabilities: [Option<Capability>; CAPS],
    pub(crate) intents: [Intent; INTENTS],
    pub(crate) events: [Event; EVENTS],
    pub(crate) actions: [ActionRecord; ACTIONS],
    pub(crate) observations: [ObservationRecord; OBSERVATIONS],
    pub(crate) checkpoints: [CheckpointRecord; CHECKPOINTS],
    pub(crate) tasks: [Task; TASKS],
    pub(crate) run_queue: [RunQueueEntry; RUN_QUEUE],
    pub(crate) messages: [MessageRecord; MESSAGES],
    pub(crate) memory_cells: [MemoryCellRecord; MEMORY_CELLS],
    pub(crate) namespace_entries: [NamespaceEntryRecord; NAMESPACE_ENTRIES],
    pub(crate) agent_len: usize,
    pub(crate) event_len: usize,
    pub(crate) action_len: usize,
    pub(crate) observation_len: usize,
    pub(crate) checkpoint_len: usize,
    pub(crate) intent_len: usize,
    pub(crate) task_len: usize,
    pub(crate) run_queue_len: usize,
    pub(crate) message_len: usize,
    pub(crate) memory_cell_len: usize,
    pub(crate) namespace_entry_len: usize,
    pub(crate) next_resource: u64,
    pub(crate) next_capability: u64,
    pub(crate) next_observation: u64,
    pub(crate) next_intent: u64,
    pub(crate) next_task: u64,
    pub(crate) next_message: u64,
    pub(crate) next_memory_cell: u64,
    pub(crate) next_namespace_entry: u64,
    pub(crate) next_sequence: u64,
}

impl<
        const AGENTS: usize,
        const RESOURCES: usize,
        const CAPS: usize,
        const EVENTS: usize,
        const ACTIONS: usize,
        const OBSERVATIONS: usize,
        const CHECKPOINTS: usize,
        const INTENTS: usize,
        const TASKS: usize,
        const RUN_QUEUE: usize,
        const MESSAGES: usize,
        const MEMORY_CELLS: usize,
        const NAMESPACE_ENTRIES: usize,
    >
    KernelCore<
        AGENTS,
        RESOURCES,
        CAPS,
        EVENTS,
        ACTIONS,
        OBSERVATIONS,
        CHECKPOINTS,
        INTENTS,
        TASKS,
        RUN_QUEUE,
        MESSAGES,
        MEMORY_CELLS,
        NAMESPACE_ENTRIES,
    >
{
    pub const fn new() -> Self {
        Self {
            agents: [AgentRecord::empty(); AGENTS],
            resources: [None; RESOURCES],
            capabilities: [None; CAPS],
            intents: [Intent::empty(); INTENTS],
            events: [Event::empty(); EVENTS],
            actions: [ActionRecord::empty(); ACTIONS],
            observations: [ObservationRecord::empty(); OBSERVATIONS],
            checkpoints: [CheckpointRecord::empty(); CHECKPOINTS],
            tasks: [Task::empty(); TASKS],
            run_queue: [RunQueueEntry::empty(); RUN_QUEUE],
            messages: [MessageRecord::empty(); MESSAGES],
            memory_cells: [MemoryCellRecord::empty(); MEMORY_CELLS],
            namespace_entries: [NamespaceEntryRecord::empty(); NAMESPACE_ENTRIES],
            agent_len: 0,
            event_len: 0,
            action_len: 0,
            observation_len: 0,
            checkpoint_len: 0,
            intent_len: 0,
            task_len: 0,
            run_queue_len: 0,
            message_len: 0,
            memory_cell_len: 0,
            namespace_entry_len: 0,
            next_resource: 1,
            next_capability: 1,
            next_observation: 1,
            next_intent: 1,
            next_task: 1,
            next_message: 1,
            next_memory_cell: 1,
            next_namespace_entry: 1,
            next_sequence: 1,
        }
    }

    pub fn events(&self) -> &[Event] {
        &self.events[..self.event_len]
    }
}

impl<
        const AGENTS: usize,
        const RESOURCES: usize,
        const CAPS: usize,
        const EVENTS: usize,
        const ACTIONS: usize,
        const OBSERVATIONS: usize,
        const CHECKPOINTS: usize,
        const INTENTS: usize,
        const TASKS: usize,
        const RUN_QUEUE: usize,
        const MESSAGES: usize,
        const MEMORY_CELLS: usize,
        const NAMESPACE_ENTRIES: usize,
    > Default
    for KernelCore<
        AGENTS,
        RESOURCES,
        CAPS,
        EVENTS,
        ACTIONS,
        OBSERVATIONS,
        CHECKPOINTS,
        INTENTS,
        TASKS,
        RUN_QUEUE,
        MESSAGES,
        MEMORY_CELLS,
        NAMESPACE_ENTRIES,
    >
{
    fn default() -> Self {
        Self::new()
    }
}
