//! Replayable signed-image Trust Policy Event constructors.
//!
//! This core-layer module binds every trust mutation to actor, Capability,
//! resource, signer metadata, rotation peer, operation, and policy generation.

use crate::{
    AgentId, AgentImageSignerEvent, AgentImageSignerId, AgentImageSignerRecord, CapabilityId,
    Event, EventKind, KernelCore, KernelError, Operation, ResourceId,
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
    pub(crate) fn record_agent_image_signer_event(
        &mut self,
        actor: AgentId,
        authority: CapabilityId,
        resource: ResourceId,
        kind: EventKind,
        operation: Operation,
        record: AgentImageSignerRecord,
        peer_signer_id: Option<AgentImageSignerId>,
    ) -> Result<Event, KernelError> {
        self.record(Event {
            agent: actor,
            kind,
            resource: Some(resource),
            capability: Some(authority),
            operation: Some(operation),
            agent_image_signer: Some(AgentImageSignerEvent::from_record(record, peer_signer_id)),
            ..Event::empty()
        })
    }
}
