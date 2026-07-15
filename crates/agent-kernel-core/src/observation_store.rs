//! Fixed-capacity kernel observation store behavior.
//!
//! This module records authorized observations without allocation and emits the
//! corresponding replayable event through the kernel event log.

use crate::{
    AgentId, CapabilityId, Event, EventKind, KernelCore, KernelError, ObservationId,
    ObservationRecord, Operation, OperationSet, ResourceId, VerificationRequirement,
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
    >
{
    pub fn observe(
        &mut self,
        agent: AgentId,
        capability: CapabilityId,
        resource: ResourceId,
    ) -> Result<Event, KernelError> {
        self.ensure_authorized(agent, capability, resource, Operation::Observe)?;
        if self.observation_len >= OBSERVATIONS {
            return Err(KernelError::ObservationStoreFull);
        }
        self.ensure_event_slots(1)?;

        let observation = ObservationId::new(self.next_observation);
        self.next_observation += 1;
        self.observations[self.observation_len] = ObservationRecord {
            id: observation,
            agent,
            resource,
            capability,
        };
        self.observation_len += 1;

        self.record(Event {
            sequence: 0,
            agent,
            kind: EventKind::Observation,
            resource: Some(resource),
            capability: Some(capability),
            source_capability: None,
            intent: None,
            intent_kind: None,
            action: None,
            observation: Some(observation),
            message: None,
            memory_cell: None,
            namespace_entry: None,
            namespace_key: None,
            namespace_object: None,
            operation: Some(Operation::Observe),
            operations: OperationSet::empty(),
            verification: VerificationRequirement::Optional,
            checkpoint: None,
            task: None,
            task_result: None,
            task_ticks: None,
            task_quantum: None,
            fault: None,
            fault_kind: None,
            fault_detail: None,
            fault_policy: None,
            fault_policy_action: None,
            waiter: None,
            signal: None,
            target_agent: None,
            driver_binding: None,
            device_event: None,
            device_event_kind: None,
            device_event_payload: None,
            driver_command: None,
            driver_command_kind: None,
            driver_command_payload: None,
            driver_command_result: None,
            driver_invocation: None,
            driver_invocation_ticks: None,
            driver_invocation_quantum: None,
            agent_image: None,
            agent_image_kind: None,
            agent_image_digest: None,
            agent_image_abi_version: None,
            agent_image_entry_version: None,
        })
    }

    pub fn observations(&self) -> &[ObservationRecord] {
        &self.observations[..self.observation_len]
    }
}
