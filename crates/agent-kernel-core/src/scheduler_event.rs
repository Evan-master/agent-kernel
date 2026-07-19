//! Deterministic scheduler event construction.
//!
//! This `agent-kernel-core` module maps task queue, dispatch, yield, and tick
//! transitions into the fixed replayable Event shape. Scheduler state and
//! capacity reservation remain in the scheduler modules; this builder performs
//! no policy selection and depends only on task lookup plus the event log.

use crate::{AgentId, Event, EventKind, KernelCore, KernelError, ResourceId, TaskId};

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
        const FAULTS: usize,
        const FAULT_HANDLERS: usize,
        const FAULT_POLICIES: usize,
        const WAITERS: usize,
        const AGENT_IMAGES: usize,
        const DRIVER_BINDINGS: usize,
        const DEVICE_EVENTS: usize,
        const DRIVER_COMMANDS: usize,
        const DRIVER_INVOCATIONS: usize,
        const RUNTIME_ADMISSIONS: usize,
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
        FAULTS,
        FAULT_HANDLERS,
        FAULT_POLICIES,
        WAITERS,
        AGENT_IMAGES,
        DRIVER_BINDINGS,
        DEVICE_EVENTS,
        DRIVER_COMMANDS,
        DRIVER_INVOCATIONS,
        RUNTIME_ADMISSIONS,
    >
{
    pub(crate) fn record_scheduler_event(
        &mut self,
        kind: EventKind,
        agent: AgentId,
        task: TaskId,
        resource: ResourceId,
        task_ticks: Option<u64>,
        task_quantum: Option<u64>,
    ) -> Result<Event, KernelError> {
        let intent = self.find_task(task)?.intent;
        let mut event = Event::empty();
        event.agent = agent;
        event.kind = kind;
        event.resource = Some(resource);
        event.intent = Some(intent);
        event.task = Some(task);
        event.task_ticks = task_ticks;
        event.task_quantum = task_quantum;
        self.record(event)
    }
}
