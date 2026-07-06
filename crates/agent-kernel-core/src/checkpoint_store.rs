//! Fixed-capacity kernel checkpoint store behavior.
//!
//! This module records authorized checkpoints, tracks rollback requests, and
//! emits replayable events without allocation or resource snapshot execution.

use crate::{
    AgentId, CapabilityId, CheckpointId, CheckpointRecord, CheckpointStatus, Event, EventKind,
    KernelCore, KernelError, Operation, OperationSet, ResourceId, VerificationRequirement,
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
    >
{
    pub fn checkpoint(
        &mut self,
        agent: AgentId,
        capability: CapabilityId,
        checkpoint: CheckpointId,
        resource: ResourceId,
    ) -> Result<Event, KernelError> {
        self.ensure_authorized(agent, capability, resource, Operation::Checkpoint)?;
        if self.find_checkpoint(checkpoint).is_ok() {
            return Err(KernelError::CheckpointAlreadyExists);
        }
        if self.checkpoint_len >= CHECKPOINTS {
            return Err(KernelError::CheckpointStoreFull);
        }
        self.ensure_event_slots(1)?;

        self.checkpoints[self.checkpoint_len] = CheckpointRecord {
            id: checkpoint,
            agent,
            resource,
            capability,
            status: CheckpointStatus::Created,
        };
        self.checkpoint_len += 1;

        self.record(Event {
            sequence: 0,
            agent,
            kind: EventKind::CheckpointCreated,
            resource: Some(resource),
            capability: Some(capability),
            source_capability: None,
            intent: None,
            intent_kind: None,
            action: None,
            observation: None,
            message: None,
            memory_cell: None,
            namespace_entry: None,
            namespace_key: None,
            namespace_object: None,
            operation: Some(Operation::Checkpoint),
            operations: OperationSet::empty(),
            verification: VerificationRequirement::Optional,
            checkpoint: Some(checkpoint),
            task: None,
            task_ticks: None,
            task_quantum: None,
            fault: None,
            fault_kind: None,
            fault_detail: None,
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

        let record = self.find_checkpoint(checkpoint)?;
        if record.resource != resource {
            return Err(KernelError::CheckpointResourceMismatch);
        }
        if record.status != CheckpointStatus::Created {
            return Err(KernelError::CheckpointStatusMismatch);
        }
        self.ensure_event_slots(1)?;

        self.find_checkpoint_mut(checkpoint)?.status = CheckpointStatus::RollbackRequested;

        self.record(Event {
            sequence: 0,
            agent,
            kind: EventKind::RollbackRequested,
            resource: Some(resource),
            capability: Some(capability),
            source_capability: None,
            intent: None,
            intent_kind: None,
            action: None,
            observation: None,
            message: None,
            memory_cell: None,
            namespace_entry: None,
            namespace_key: None,
            namespace_object: None,
            operation: Some(Operation::Rollback),
            operations: OperationSet::empty(),
            verification: VerificationRequirement::Optional,
            checkpoint: Some(checkpoint),
            task: None,
            task_ticks: None,
            task_quantum: None,
            fault: None,
            fault_kind: None,
            fault_detail: None,
            target_agent: None,
        })
    }

    pub fn checkpoints(&self) -> &[CheckpointRecord] {
        &self.checkpoints[..self.checkpoint_len]
    }

    pub(crate) fn find_checkpoint(
        &self,
        id: CheckpointId,
    ) -> Result<CheckpointRecord, KernelError> {
        for checkpoint in self.checkpoints() {
            if checkpoint.id == id {
                return Ok(*checkpoint);
            }
        }

        Err(KernelError::CheckpointNotFound)
    }

    fn find_checkpoint_mut(
        &mut self,
        id: CheckpointId,
    ) -> Result<&mut CheckpointRecord, KernelError> {
        for checkpoint in &mut self.checkpoints[..self.checkpoint_len] {
            if checkpoint.id == id {
                return Ok(checkpoint);
            }
        }

        Err(KernelError::CheckpointNotFound)
    }
}
