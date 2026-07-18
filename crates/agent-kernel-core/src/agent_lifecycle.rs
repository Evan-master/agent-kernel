//! Trusted bootstrap Agent lifecycle transitions.
//!
//! This core-layer module preserves the original host and bootstrap lifecycle
//! API. These calls carry no runtime actor authority; native Agent Calls use
//! the capability-authorized management module.

use crate::{AgentId, AgentStatus, Event, EventKind, KernelCore, KernelError};

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
    pub fn suspend_agent(&mut self, agent: AgentId) -> Result<Event, KernelError> {
        match self.find_agent(agent)?.status {
            AgentStatus::Active => self.transition_trusted_agent(
                agent,
                AgentStatus::Suspended,
                EventKind::AgentSuspended,
            ),
            AgentStatus::Suspended => Err(KernelError::AgentStatusMismatch),
            AgentStatus::Retired => Err(KernelError::AgentRetired),
        }
    }

    pub fn resume_agent(&mut self, agent: AgentId) -> Result<Event, KernelError> {
        match self.find_agent(agent)?.status {
            AgentStatus::Suspended => {
                self.transition_trusted_agent(agent, AgentStatus::Active, EventKind::AgentResumed)
            }
            AgentStatus::Active => Err(KernelError::AgentStatusMismatch),
            AgentStatus::Retired => Err(KernelError::AgentRetired),
        }
    }

    pub fn retire_agent(&mut self, agent: AgentId) -> Result<Event, KernelError> {
        match self.find_agent(agent)?.status {
            AgentStatus::Active | AgentStatus::Suspended => {
                self.transition_trusted_agent(agent, AgentStatus::Retired, EventKind::AgentRetired)
            }
            AgentStatus::Retired => Err(KernelError::AgentRetired),
        }
    }

    fn transition_trusted_agent(
        &mut self,
        agent: AgentId,
        status: AgentStatus,
        kind: EventKind,
    ) -> Result<Event, KernelError> {
        self.ensure_event_slots(1)?;
        self.find_agent_mut(agent)?.status = status;
        self.record_agent_event(kind, agent, agent, None, None, None)
    }
}
