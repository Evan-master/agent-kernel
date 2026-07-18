//! Fixed-capacity Agent registration and lookup.
//!
//! This core-layer module owns deterministic trusted registration, record
//! insertion, and read-only lookup. Lifecycle policy and event construction
//! live in neighboring modules so this store stays focused and auditable.

use crate::{
    AgentExecutionContext, AgentId, AgentRecord, AgentStatus, Event, EventKind, KernelCore,
    KernelError,
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
    pub fn register_agent(&mut self, agent: AgentId) -> Result<Event, KernelError> {
        self.ensure_agent_registration_available(agent)?;
        self.ensure_event_slots(1)?;
        self.insert_agent_record(agent, None, None);
        self.record_agent_event(EventKind::AgentRegistered, agent, agent, None, None, None)
    }

    pub fn agents(&self) -> &[AgentRecord] {
        &self.agents[..self.agent_len]
    }

    pub(crate) fn find_agent(&self, id: AgentId) -> Result<AgentRecord, KernelError> {
        self.agents()
            .iter()
            .find(|agent| agent.id == id)
            .copied()
            .ok_or(KernelError::AgentNotFound)
    }

    pub(crate) fn ensure_agent_active(&self, id: AgentId) -> Result<AgentRecord, KernelError> {
        let agent = self.find_agent(id)?;
        match agent.status {
            AgentStatus::Active => Ok(agent),
            AgentStatus::Suspended => Err(KernelError::AgentSuspended),
            AgentStatus::Retired => Err(KernelError::AgentRetired),
        }
    }

    pub(crate) fn ensure_agent_registration_available(
        &self,
        agent: AgentId,
    ) -> Result<(), KernelError> {
        if self.find_agent(agent).is_ok() {
            return Err(KernelError::AgentAlreadyExists);
        }
        if self.agent_len >= AGENTS {
            return Err(KernelError::AgentStoreFull);
        }
        Ok(())
    }

    pub(crate) fn insert_agent_record(
        &mut self,
        agent: AgentId,
        manager: Option<AgentId>,
        management_resource: Option<crate::ResourceId>,
    ) {
        let index = self.agent_len;
        self.agents[index] = AgentRecord {
            id: agent,
            status: AgentStatus::Active,
            manager,
            management_resource,
        };
        self.execution_contexts[index] = AgentExecutionContext::idle(agent);
        self.agent_len += 1;
    }

    pub(crate) fn find_agent_mut(&mut self, id: AgentId) -> Result<&mut AgentRecord, KernelError> {
        self.agents[..self.agent_len]
            .iter_mut()
            .find(|agent| agent.id == id)
            .ok_or(KernelError::AgentNotFound)
    }
}
