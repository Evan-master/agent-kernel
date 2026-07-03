//! Fixed-capacity Agent Kernel core state machine.
//!
//! This module owns resource registration, capability grants, authorization,
//! event recording, checkpoint creation, and rollback requests. It performs no
//! host I/O and keeps state deterministic for replay and supervisor inspection.

use crate::{
    ActionId, AgentId, Capability, CapabilityId, CheckpointId, Event, EventKind, Intent,
    KernelError, Operation, OperationSet, Resource, ResourceId, RunQueueEntry, Task,
};

#[derive(Debug)]
pub struct KernelCore<
    const RESOURCES: usize,
    const CAPS: usize,
    const EVENTS: usize,
    const INTENTS: usize,
    const TASKS: usize,
    const RUN_QUEUE: usize,
> {
    pub(crate) resources: [Option<Resource>; RESOURCES],
    pub(crate) capabilities: [Option<Capability>; CAPS],
    pub(crate) intents: [Intent; INTENTS],
    pub(crate) events: [Event; EVENTS],
    pub(crate) tasks: [Task; TASKS],
    pub(crate) run_queue: [RunQueueEntry; RUN_QUEUE],
    pub(crate) event_len: usize,
    pub(crate) intent_len: usize,
    pub(crate) task_len: usize,
    pub(crate) run_queue_len: usize,
    pub(crate) next_resource: u64,
    pub(crate) next_capability: u64,
    pub(crate) next_intent: u64,
    pub(crate) next_task: u64,
    pub(crate) next_sequence: u64,
}

impl<
        const RESOURCES: usize,
        const CAPS: usize,
        const EVENTS: usize,
        const INTENTS: usize,
        const TASKS: usize,
        const RUN_QUEUE: usize,
    > KernelCore<RESOURCES, CAPS, EVENTS, INTENTS, TASKS, RUN_QUEUE>
{
    pub const fn new() -> Self {
        Self {
            resources: [None; RESOURCES],
            capabilities: [None; CAPS],
            intents: [Intent::empty(); INTENTS],
            events: [Event::empty(); EVENTS],
            tasks: [Task::empty(); TASKS],
            run_queue: [RunQueueEntry::empty(); RUN_QUEUE],
            event_len: 0,
            intent_len: 0,
            task_len: 0,
            run_queue_len: 0,
            next_resource: 1,
            next_capability: 1,
            next_intent: 1,
            next_task: 1,
            next_sequence: 1,
        }
    }

    pub fn authorize(
        &mut self,
        agent: AgentId,
        capability: CapabilityId,
        resource: ResourceId,
        operation: Operation,
    ) -> Result<Event, KernelError> {
        self.ensure_authorized(agent, capability, resource, operation)?;

        self.record(resource_event(
            agent,
            event_kind(operation),
            resource,
            capability,
            Some(operation),
            None,
            None,
        ))
    }

    pub fn act(
        &mut self,
        agent: AgentId,
        capability: CapabilityId,
        action: ActionId,
        resource: ResourceId,
    ) -> Result<Event, KernelError> {
        self.ensure_authorized(agent, capability, resource, Operation::Act)?;
        self.record(resource_event(
            agent,
            EventKind::ActionExecuted,
            resource,
            capability,
            Some(Operation::Act),
            Some(action),
            None,
        ))
    }

    pub fn verify(
        &mut self,
        agent: AgentId,
        capability: CapabilityId,
        action: ActionId,
        resource: ResourceId,
    ) -> Result<Event, KernelError> {
        self.ensure_authorized(agent, capability, resource, Operation::Verify)?;
        self.record(resource_event(
            agent,
            EventKind::VerificationRequested,
            resource,
            capability,
            Some(Operation::Verify),
            Some(action),
            None,
        ))
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
            None,
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
            None,
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
        const INTENTS: usize,
        const TASKS: usize,
        const RUN_QUEUE: usize,
    > Default for KernelCore<RESOURCES, CAPS, EVENTS, INTENTS, TASKS, RUN_QUEUE>
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
    action: Option<ActionId>,
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
        action,
        operation,
        operations: OperationSet::empty(),
        verification: crate::VerificationRequirement::Optional,
        checkpoint,
        task: None,
        target_agent: None,
    }
}

const fn event_kind(operation: Operation) -> EventKind {
    match operation {
        Operation::Observe => EventKind::Observation,
        Operation::Act => EventKind::ActionExecuted,
        Operation::Verify => EventKind::VerificationRequested,
        Operation::Checkpoint => EventKind::CheckpointCreated,
        Operation::Rollback => EventKind::RollbackRequested,
        Operation::Delegate => EventKind::DelegationRequested,
    }
}
