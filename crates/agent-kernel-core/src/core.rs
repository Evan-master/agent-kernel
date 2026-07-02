//! Fixed-capacity Agent Kernel core state machine.
//!
//! This module owns resource registration, capability grants, authorization,
//! event recording, checkpoint creation, and rollback requests. It performs no
//! host I/O and keeps state deterministic for replay and supervisor inspection.

use crate::{
    AgentId, Capability, CapabilityId, CheckpointId, Event, EventKind, KernelError, Operation,
    OperationSet, Resource, ResourceId, ResourceKind,
};

#[derive(Debug)]
pub struct KernelCore<const RESOURCES: usize, const CAPS: usize, const EVENTS: usize> {
    resources: [Option<Resource>; RESOURCES],
    capabilities: [Option<Capability>; CAPS],
    events: [Event; EVENTS],
    event_len: usize,
    next_resource: u64,
    next_capability: u64,
    next_sequence: u64,
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
            operation: Some(operation),
            checkpoint: None,
        })
    }

    pub fn revoke_capability(&mut self, capability: CapabilityId) -> Result<(), KernelError> {
        let cap = self.find_capability_mut(capability)?;
        cap.revoked = true;
        Ok(())
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
            operation: Some(Operation::Checkpoint),
            checkpoint: Some(checkpoint),
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
            operation: Some(Operation::Rollback),
            checkpoint: Some(checkpoint),
        })
    }

    pub fn events(&self) -> &[Event] {
        &self.events[..self.event_len]
    }

    fn record(&mut self, event: Event) -> Result<Event, KernelError> {
        if self.event_len >= EVENTS {
            return Err(KernelError::EventLogFull);
        }

        let mut event = event;
        event.sequence = self.next_sequence;
        self.next_sequence += 1;
        self.events[self.event_len] = event;
        self.event_len += 1;
        Ok(event)
    }

    fn ensure_authorized(
        &self,
        agent: AgentId,
        capability: CapabilityId,
        resource: ResourceId,
        operation: Operation,
    ) -> Result<(), KernelError> {
        self.find_resource(resource)?;
        let cap = self.find_capability(capability)?;

        if cap.revoked {
            return Err(KernelError::CapabilityRevoked);
        }
        if cap.agent != agent {
            return Err(KernelError::AgentMismatch);
        }
        if cap.resource != resource {
            return Err(KernelError::ResourceMismatch);
        }
        if !cap.operations.allows(operation) {
            return Err(KernelError::OperationDenied);
        }

        Ok(())
    }

    fn find_resource(&self, id: ResourceId) -> Result<Resource, KernelError> {
        self.resources
            .iter()
            .flatten()
            .find(|resource| resource.id == id)
            .copied()
            .ok_or(KernelError::ResourceNotFound)
    }

    fn find_capability(&self, id: CapabilityId) -> Result<Capability, KernelError> {
        self.capabilities
            .iter()
            .flatten()
            .find(|capability| capability.id == id)
            .copied()
            .ok_or(KernelError::CapabilityNotFound)
    }

    fn find_capability_mut(&mut self, id: CapabilityId) -> Result<&mut Capability, KernelError> {
        self.capabilities
            .iter_mut()
            .flatten()
            .find(|capability| capability.id == id)
            .ok_or(KernelError::CapabilityNotFound)
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
