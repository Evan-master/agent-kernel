//! DMA lifecycle Event construction.
//!
//! This core module centralizes replay-visible DMA mutations while reusing the
//! common Event shape. It performs no validation beyond Event-log capacity.

use crate::{
    AgentId, CapabilityId, Event, EventKind, KernelCore, KernelError, Operation, ResourceId,
};

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
    pub(crate) fn record_dma_event(
        &mut self,
        kind: EventKind,
        agent: AgentId,
        resource: ResourceId,
        capability: CapabilityId,
        source_capability: Option<CapabilityId>,
    ) -> Result<Event, KernelError> {
        self.record_dma_event_with_operation(
            kind,
            agent,
            resource,
            capability,
            source_capability,
            Operation::Act,
        )
    }

    pub(crate) fn record_dma_event_with_operation(
        &mut self,
        kind: EventKind,
        agent: AgentId,
        resource: ResourceId,
        capability: CapabilityId,
        source_capability: Option<CapabilityId>,
        operation: Operation,
    ) -> Result<Event, KernelError> {
        let mut event = Event::empty();
        event.agent = agent;
        event.kind = kind;
        event.resource = Some(resource);
        event.capability = Some(capability);
        event.source_capability = source_capability;
        event.operation = Some(operation);
        self.record(event)
    }
}
