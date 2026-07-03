//! Fixed-capacity Agent Kernel core state machine.
//!
//! This module owns resource registration, capability grants, authorization,
//! event recording, checkpoint creation, and rollback requests. It performs no
//! host I/O and keeps state deterministic for replay and supervisor inspection.

use crate::{
    ActionRecord, AgentId, Capability, CapabilityId, CheckpointId, Event, EventKind, Intent,
    KernelError, ObservationRecord, Operation, OperationSet, Resource, ResourceId, RunQueueEntry,
    Task,
};

#[derive(Debug)]
pub struct KernelCore<
    const RESOURCES: usize,
    const CAPS: usize,
    const EVENTS: usize,
    const ACTIONS: usize,
    const OBSERVATIONS: usize,
    const INTENTS: usize,
    const TASKS: usize,
    const RUN_QUEUE: usize,
> {
    pub(crate) resources: [Option<Resource>; RESOURCES],
    pub(crate) capabilities: [Option<Capability>; CAPS],
    pub(crate) intents: [Intent; INTENTS],
    pub(crate) events: [Event; EVENTS],
    pub(crate) actions: [ActionRecord; ACTIONS],
    pub(crate) observations: [ObservationRecord; OBSERVATIONS],
    pub(crate) tasks: [Task; TASKS],
    pub(crate) run_queue: [RunQueueEntry; RUN_QUEUE],
    pub(crate) event_len: usize,
    pub(crate) action_len: usize,
    pub(crate) observation_len: usize,
    pub(crate) intent_len: usize,
    pub(crate) task_len: usize,
    pub(crate) run_queue_len: usize,
    pub(crate) next_resource: u64,
    pub(crate) next_capability: u64,
    pub(crate) next_observation: u64,
    pub(crate) next_intent: u64,
    pub(crate) next_task: u64,
    pub(crate) next_sequence: u64,
}

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
    pub const fn new() -> Self {
        Self {
            resources: [None; RESOURCES],
            capabilities: [None; CAPS],
            intents: [Intent::empty(); INTENTS],
            events: [Event::empty(); EVENTS],
            actions: [ActionRecord::empty(); ACTIONS],
            observations: [ObservationRecord::empty(); OBSERVATIONS],
            tasks: [Task::empty(); TASKS],
            run_queue: [RunQueueEntry::empty(); RUN_QUEUE],
            event_len: 0,
            action_len: 0,
            observation_len: 0,
            intent_len: 0,
            task_len: 0,
            run_queue_len: 0,
            next_resource: 1,
            next_capability: 1,
            next_observation: 1,
            next_intent: 1,
            next_task: 1,
            next_sequence: 1,
        }
    }

    pub fn checkpoint(
        &mut self,
        agent: AgentId,
        capability: CapabilityId,
        checkpoint: CheckpointId,
        resource: ResourceId,
    ) -> Result<Event, KernelError> {
        self.ensure_authorized(agent, capability, resource, Operation::Checkpoint)?;
        self.record(resource_event(
            agent,
            EventKind::CheckpointCreated,
            resource,
            capability,
            Some(Operation::Checkpoint),
            Some(checkpoint),
        ))
    }

    pub fn rollback(
        &mut self,
        agent: AgentId,
        capability: CapabilityId,
        checkpoint: CheckpointId,
        resource: ResourceId,
    ) -> Result<Event, KernelError> {
        self.ensure_authorized(agent, capability, resource, Operation::Rollback)?;
        self.record(resource_event(
            agent,
            EventKind::RollbackRequested,
            resource,
            capability,
            Some(Operation::Rollback),
            Some(checkpoint),
        ))
    }

    pub fn events(&self) -> &[Event] {
        &self.events[..self.event_len]
    }
}

impl<
        const RESOURCES: usize,
        const CAPS: usize,
        const EVENTS: usize,
        const ACTIONS: usize,
        const OBSERVATIONS: usize,
        const INTENTS: usize,
        const TASKS: usize,
        const RUN_QUEUE: usize,
    > Default
    for KernelCore<RESOURCES, CAPS, EVENTS, ACTIONS, OBSERVATIONS, INTENTS, TASKS, RUN_QUEUE>
{
    fn default() -> Self {
        Self::new()
    }
}

fn resource_event(
    agent: AgentId,
    kind: EventKind,
    resource: ResourceId,
    capability: CapabilityId,
    operation: Option<Operation>,
    checkpoint: Option<CheckpointId>,
) -> Event {
    Event {
        sequence: 0,
        agent,
        kind,
        resource: Some(resource),
        capability: Some(capability),
        source_capability: None,
        intent: None,
        intent_kind: None,
        action: None,
        observation: None,
        operation,
        operations: OperationSet::empty(),
        verification: crate::VerificationRequirement::Optional,
        checkpoint,
        task: None,
        target_agent: None,
    }
}
