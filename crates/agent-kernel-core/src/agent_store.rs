//! Fixed-capacity kernel agent registry.
//!
//! This module owns deterministic agent registration, lifecycle status changes,
//! active-agent authority checks, and read-only inspection. It emits replayable
//! events without allocation and keeps failure paths atomic with respect to both
//! the registry and the event log.

use crate::{
    AgentExecutionContext, AgentId, AgentRecord, AgentStatus, Event, EventKind, KernelCore,
    KernelError, OperationSet, VerificationRequirement,
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
    >
{
    pub fn register_agent(&mut self, agent: AgentId) -> Result<Event, KernelError> {
        if self.find_agent(agent).is_ok() {
            return Err(KernelError::AgentAlreadyExists);
        }
        if self.agent_len >= AGENTS {
            return Err(KernelError::AgentStoreFull);
        }
        self.ensure_event_slots(1)?;

        let index = self.agent_len;
        self.agents[index] = AgentRecord {
            id: agent,
            status: AgentStatus::Active,
        };
        self.execution_contexts[index] = AgentExecutionContext::idle(agent);
        self.agent_len += 1;

        self.record(Event {
            sequence: 0,
            agent,
            kind: EventKind::AgentRegistered,
            resource: None,
            capability: None,
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
            operation: None,
            operations: OperationSet::empty(),
            verification: VerificationRequirement::Optional,
            checkpoint: None,
            task: None,
            task_ticks: None,
            task_quantum: None,
            fault: None,
            fault_kind: None,
            fault_detail: None,
            fault_policy: None,
            fault_policy_action: None,
            waiter: None,
            signal: None,
            target_agent: Some(agent),
            driver_binding: None,
            device_event: None,
            device_event_kind: None,
            device_event_payload: None,
            agent_image: None,
            agent_image_kind: None,
            agent_image_digest: None,
            agent_image_abi_version: None,
            agent_image_entry_version: None,
        })
    }

    pub fn suspend_agent(&mut self, agent: AgentId) -> Result<Event, KernelError> {
        let record = self.find_agent(agent)?;
        match record.status {
            AgentStatus::Active => self.set_agent_status(agent, AgentStatus::Suspended),
            AgentStatus::Suspended => Err(KernelError::AgentStatusMismatch),
            AgentStatus::Retired => Err(KernelError::AgentRetired),
        }?;
        self.record_agent_lifecycle_event(EventKind::AgentSuspended, agent)
    }

    pub fn resume_agent(&mut self, agent: AgentId) -> Result<Event, KernelError> {
        let record = self.find_agent(agent)?;
        match record.status {
            AgentStatus::Suspended => self.set_agent_status(agent, AgentStatus::Active),
            AgentStatus::Active => Err(KernelError::AgentStatusMismatch),
            AgentStatus::Retired => Err(KernelError::AgentRetired),
        }?;
        self.record_agent_lifecycle_event(EventKind::AgentResumed, agent)
    }

    pub fn retire_agent(&mut self, agent: AgentId) -> Result<Event, KernelError> {
        let record = self.find_agent(agent)?;
        match record.status {
            AgentStatus::Active | AgentStatus::Suspended => {
                self.set_agent_status(agent, AgentStatus::Retired)
            }
            AgentStatus::Retired => Err(KernelError::AgentRetired),
        }?;
        self.record_agent_lifecycle_event(EventKind::AgentRetired, agent)
    }

    pub fn agents(&self) -> &[AgentRecord] {
        &self.agents[..self.agent_len]
    }

    pub(crate) fn find_agent(&self, id: AgentId) -> Result<AgentRecord, KernelError> {
        for agent in self.agents() {
            if agent.id == id {
                return Ok(*agent);
            }
        }

        Err(KernelError::AgentNotFound)
    }

    pub(crate) fn ensure_agent_active(&self, id: AgentId) -> Result<AgentRecord, KernelError> {
        let agent = self.find_agent(id)?;
        match agent.status {
            AgentStatus::Active => Ok(agent),
            AgentStatus::Suspended => Err(KernelError::AgentSuspended),
            AgentStatus::Retired => Err(KernelError::AgentRetired),
        }
    }

    fn find_agent_mut(&mut self, id: AgentId) -> Result<&mut AgentRecord, KernelError> {
        for agent in &mut self.agents[..self.agent_len] {
            if agent.id == id {
                return Ok(agent);
            }
        }

        Err(KernelError::AgentNotFound)
    }

    fn set_agent_status(&mut self, agent: AgentId, status: AgentStatus) -> Result<(), KernelError> {
        self.ensure_event_slots(1)?;
        self.find_agent_mut(agent)?.status = status;
        Ok(())
    }

    fn record_agent_lifecycle_event(
        &mut self,
        kind: EventKind,
        agent: AgentId,
    ) -> Result<Event, KernelError> {
        self.record(Event {
            sequence: 0,
            agent,
            kind,
            resource: None,
            capability: None,
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
            operation: None,
            operations: OperationSet::empty(),
            verification: VerificationRequirement::Optional,
            checkpoint: None,
            task: None,
            task_ticks: None,
            task_quantum: None,
            fault: None,
            fault_kind: None,
            fault_detail: None,
            fault_policy: None,
            fault_policy_action: None,
            waiter: None,
            signal: None,
            target_agent: Some(agent),
            driver_binding: None,
            device_event: None,
            device_event_kind: None,
            device_event_payload: None,
            agent_image: None,
            agent_image_kind: None,
            agent_image_digest: None,
            agent_image_abi_version: None,
            agent_image_entry_version: None,
        })
    }
}
