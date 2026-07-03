//! Fixed-capacity kernel agent registry.
//!
//! This module owns deterministic agent registration and read-only inspection.
//! It emits replayable events without allocation and keeps registration failure
//! paths atomic with respect to both the registry and the event log.

use crate::{
    AgentId, AgentRecord, AgentStatus, Event, EventKind, KernelCore, KernelError, OperationSet,
    VerificationRequirement,
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

        self.agents[self.agent_len] = AgentRecord {
            id: agent,
            status: AgentStatus::Active,
        };
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
            operation: None,
            operations: OperationSet::empty(),
            verification: VerificationRequirement::Optional,
            checkpoint: None,
            task: None,
            target_agent: Some(agent),
        })
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
}
