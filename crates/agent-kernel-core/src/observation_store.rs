//! Fixed-capacity kernel observation store behavior.
//!
//! This module records authorized observations without allocation and emits the
//! corresponding replayable event through the kernel event log.

use crate::{
    AgentId, CapabilityId, Event, EventKind, KernelCore, KernelError, ObservationId,
    ObservationRecord, Operation, OperationSet, ResourceId, VerificationRequirement,
};

impl<
        const RESOURCES: usize,
        const CAPS: usize,
        const EVENTS: usize,
        const ACTIONS: usize,
        const OBSERVATIONS: usize,
        const INTENTS: usize,
        const TASKS: usize,
        const RUN_QUEUE: usize,
    > KernelCore<RESOURCES, CAPS, EVENTS, ACTIONS, OBSERVATIONS, INTENTS, TASKS, RUN_QUEUE>
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
            operation: Some(Operation::Observe),
            operations: OperationSet::empty(),
            verification: VerificationRequirement::Optional,
            checkpoint: None,
            task: None,
            target_agent: None,
        })
    }

    pub fn observations(&self) -> &[ObservationRecord] {
        &self.observations[..self.observation_len]
    }
}
