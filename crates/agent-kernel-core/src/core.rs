//! Fixed-capacity Agent Kernel core state machine.
//!
//! This module owns resource registration, capability grants, authorization,
//! event recording, checkpoint creation, and rollback requests. It performs no
//! host I/O and keeps state deterministic for replay and supervisor inspection.

use crate::{
    ActionId, AgentId, Capability, CapabilityId, CheckpointId, Event, EventKind, KernelError,
    Operation, OperationSet, Resource, ResourceId, RunQueueEntry, Task,
};

#[derive(Debug)]
pub struct KernelCore<
    const RESOURCES: usize,
    const CAPS: usize,
    const EVENTS: usize,
    const TASKS: usize,
    const RUN_QUEUE: usize,
> {
    pub(crate) resources: [Option<Resource>; RESOURCES],
    pub(crate) capabilities: [Option<Capability>; CAPS],
    pub(crate) events: [Event; EVENTS],
    pub(crate) tasks: [Task; TASKS],
    pub(crate) run_queue: [RunQueueEntry; RUN_QUEUE],
    pub(crate) event_len: usize,
    pub(crate) task_len: usize,
    pub(crate) run_queue_len: usize,
    pub(crate) next_resource: u64,
    pub(crate) next_capability: u64,
    pub(crate) next_task: u64,
    pub(crate) next_sequence: u64,
}

impl<
        const RESOURCES: usize,
        const CAPS: usize,
        const EVENTS: usize,
        const TASKS: usize,
        const RUN_QUEUE: usize,
    > KernelCore<RESOURCES, CAPS, EVENTS, TASKS, RUN_QUEUE>
{
    pub const fn new() -> Self {
        Self {
            resources: [None; RESOURCES],
            capabilities: [None; CAPS],
            events: [Event::empty(); EVENTS],
            tasks: [Task::empty(); TASKS],
            run_queue: [RunQueueEntry::empty(); RUN_QUEUE],
            event_len: 0,
            task_len: 0,
            run_queue_len: 0,
            next_resource: 1,
            next_capability: 1,
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

        self.record(Event {
            sequence: self.next_sequence,
            agent,
            kind: event_kind(operation),
            resource: Some(resource),
            capability: Some(capability),
            source_capability: None,
            action: None,
            operation: Some(operation),
            operations: OperationSet::empty(),
            checkpoint: None,
            task: None,
            target_agent: None,
        })
    }

    pub fn act(
        &mut self,
        agent: AgentId,
        capability: CapabilityId,
        action: ActionId,
        resource: ResourceId,
    ) -> Result<Event, KernelError> {
        self.ensure_authorized(agent, capability, resource, Operation::Act)?;
        self.record(Event {
            sequence: self.next_sequence,
            agent,
            kind: EventKind::ActionExecuted,
            resource: Some(resource),
            capability: Some(capability),
            source_capability: None,
            action: Some(action),
            operation: Some(Operation::Act),
            operations: OperationSet::empty(),
            checkpoint: None,
            task: None,
            target_agent: None,
        })
    }

    pub fn verify(
        &mut self,
        agent: AgentId,
        capability: CapabilityId,
        action: ActionId,
        resource: ResourceId,
    ) -> Result<Event, KernelError> {
        self.ensure_authorized(agent, capability, resource, Operation::Verify)?;
        self.record(Event {
            sequence: self.next_sequence,
            agent,
            kind: EventKind::VerificationRequested,
            resource: Some(resource),
            capability: Some(capability),
            source_capability: None,
            action: Some(action),
            operation: Some(Operation::Verify),
            operations: OperationSet::empty(),
            checkpoint: None,
            task: None,
            target_agent: None,
        })
    }

    pub fn checkpoint(
        &mut self,
        agent: AgentId,
        capability: CapabilityId,
        checkpoint: CheckpointId,
        resource: ResourceId,
    ) -> Result<Event, KernelError> {
        self.ensure_authorized(agent, capability, resource, Operation::Checkpoint)?;
        self.record(Event {
            sequence: self.next_sequence,
            agent,
            kind: EventKind::CheckpointCreated,
            resource: Some(resource),
            capability: Some(capability),
            source_capability: None,
            action: None,
            operation: Some(Operation::Checkpoint),
            operations: OperationSet::empty(),
            checkpoint: Some(checkpoint),
            task: None,
            target_agent: None,
        })
    }

    pub fn rollback(
        &mut self,
        agent: AgentId,
        capability: CapabilityId,
        checkpoint: CheckpointId,
        resource: ResourceId,
    ) -> Result<Event, KernelError> {
        self.ensure_authorized(agent, capability, resource, Operation::Rollback)?;
        self.record(Event {
            sequence: self.next_sequence,
            agent,
            kind: EventKind::RollbackRequested,
            resource: Some(resource),
            capability: Some(capability),
            source_capability: None,
            action: None,
            operation: Some(Operation::Rollback),
            operations: OperationSet::empty(),
            checkpoint: Some(checkpoint),
            task: None,
            target_agent: None,
        })
    }

    pub fn events(&self) -> &[Event] {
        &self.events[..self.event_len]
    }
}

impl<
        const RESOURCES: usize,
        const CAPS: usize,
        const EVENTS: usize,
        const TASKS: usize,
        const RUN_QUEUE: usize,
    > Default for KernelCore<RESOURCES, CAPS, EVENTS, TASKS, RUN_QUEUE>
{
    fn default() -> Self {
        Self::new()
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
