//! Fixed-capacity Agent Kernel core state machine.
//!
//! This module owns resource registration, capability grants, authorization,
//! event recording, checkpoint creation, and rollback requests. It performs no
//! host I/O and keeps state deterministic for replay and supervisor inspection.

use crate::{
    ActionId, AgentId, Capability, CapabilityId, CheckpointId, Event, EventKind, KernelError,
    Operation, OperationSet, Resource, ResourceId, ResourceKind, TaskId,
};

#[derive(Debug)]
pub struct KernelCore<const RESOURCES: usize, const CAPS: usize, const EVENTS: usize> {
    pub(crate) resources: [Option<Resource>; RESOURCES],
    pub(crate) capabilities: [Option<Capability>; CAPS],
    pub(crate) events: [Event; EVENTS],
    pub(crate) event_len: usize,
    pub(crate) next_resource: u64,
    pub(crate) next_capability: u64,
    pub(crate) next_sequence: u64,
}

impl<const RESOURCES: usize, const CAPS: usize, const EVENTS: usize>
    KernelCore<RESOURCES, CAPS, EVENTS>
{
    pub const fn new() -> Self {
        Self {
            resources: [None; RESOURCES],
            capabilities: [None; CAPS],
            events: [Event::empty(); EVENTS],
            event_len: 0,
            next_resource: 1,
            next_capability: 1,
            next_sequence: 1,
        }
    }

    pub fn register_resource(
        &mut self,
        kind: ResourceKind,
        parent: Option<ResourceId>,
    ) -> Result<ResourceId, KernelError> {
        if let Some(parent_id) = parent {
            self.find_resource(parent_id)?;
        }

        let slot = self
            .resources
            .iter_mut()
            .find(|resource| resource.is_none())
            .ok_or(KernelError::ResourceStoreFull)?;
        let id = ResourceId::new(self.next_resource);
        self.next_resource += 1;
        *slot = Some(Resource { id, kind, parent });
        Ok(id)
    }

    pub fn grant_capability(
        &mut self,
        agent: AgentId,
        resource: ResourceId,
        operations: OperationSet,
    ) -> Result<CapabilityId, KernelError> {
        self.find_resource(resource)?;

        let slot = self
            .capabilities
            .iter_mut()
            .find(|capability| capability.is_none())
            .ok_or(KernelError::CapabilityStoreFull)?;
        let id = CapabilityId::new(self.next_capability);
        self.next_capability += 1;
        *slot = Some(Capability {
            id,
            agent,
            resource,
            operations,
            revoked: false,
        });
        Ok(id)
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
            action: None,
            operation: Some(operation),
            checkpoint: None,
            task: None,
            target_agent: None,
        })
    }

    pub fn revoke_capability(&mut self, capability: CapabilityId) -> Result<(), KernelError> {
        let cap = self.find_capability_mut(capability)?;
        cap.revoked = true;
        Ok(())
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
            action: Some(action),
            operation: Some(Operation::Act),
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
            action: Some(action),
            operation: Some(Operation::Verify),
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
            action: None,
            operation: Some(Operation::Checkpoint),
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
            action: None,
            operation: Some(Operation::Rollback),
            checkpoint: Some(checkpoint),
            task: None,
            target_agent: None,
        })
    }

    pub fn delegate(
        &mut self,
        agent: AgentId,
        capability: CapabilityId,
        task: TaskId,
        resource: ResourceId,
        target_agent: AgentId,
    ) -> Result<Event, KernelError> {
        self.ensure_authorized(agent, capability, resource, Operation::Delegate)?;
        self.record(Event {
            sequence: self.next_sequence,
            agent,
            kind: EventKind::DelegationRequested,
            resource: Some(resource),
            capability: Some(capability),
            action: None,
            operation: Some(Operation::Delegate),
            checkpoint: None,
            task: Some(task),
            target_agent: Some(target_agent),
        })
    }

    pub fn events(&self) -> &[Event] {
        &self.events[..self.event_len]
    }
}

impl<const RESOURCES: usize, const CAPS: usize, const EVENTS: usize> Default
    for KernelCore<RESOURCES, CAPS, EVENTS>
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
