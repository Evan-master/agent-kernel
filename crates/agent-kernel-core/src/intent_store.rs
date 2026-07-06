//! Fixed-capacity intent declaration and lookup.
//!
//! This module belongs to `agent-kernel-core`. It owns deterministic intent
//! allocation, capability-gated declaration, and read-only inspection without
//! storing natural language, prompts, or host planning state.

use crate::{
    AgentId, CapabilityId, Event, EventKind, Intent, IntentId, IntentKind, IntentStatus,
    KernelCore, KernelError, OperationSet, ResourceId, VerificationRequirement,
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
    >
{
    pub fn declare_intent(
        &mut self,
        agent: AgentId,
        capability: CapabilityId,
        resource: ResourceId,
        kind: IntentKind,
        verification: VerificationRequirement,
    ) -> Result<IntentId, KernelError> {
        let operation = kind.required_operation();
        self.ensure_authorized(agent, capability, resource, operation)?;
        if self.intent_len >= INTENTS {
            return Err(KernelError::IntentStoreFull);
        }
        self.ensure_event_slots(1)?;

        let intent = IntentId::new(self.next_intent);
        self.next_intent += 1;
        self.intents[self.intent_len] = Intent {
            id: intent,
            owner: agent,
            resource,
            kind,
            status: IntentStatus::Declared,
            verification,
        };
        self.intent_len += 1;
        self.record_intent_event(agent, capability, resource, intent, kind, verification)?;
        Ok(intent)
    }

    pub fn intents(&self) -> &[Intent] {
        &self.intents[..self.intent_len]
    }

    pub(crate) fn find_intent(&self, id: IntentId) -> Result<Intent, KernelError> {
        self.intents()
            .iter()
            .find(|intent| intent.id == id)
            .copied()
            .ok_or(KernelError::IntentNotFound)
    }

    fn find_intent_mut(&mut self, id: IntentId) -> Result<&mut Intent, KernelError> {
        self.intents[..self.intent_len]
            .iter_mut()
            .find(|intent| intent.id == id)
            .ok_or(KernelError::IntentNotFound)
    }

    pub(crate) fn ensure_intent_status(
        &self,
        id: IntentId,
        expected: IntentStatus,
    ) -> Result<Intent, KernelError> {
        let intent = self.find_intent(id)?;
        if intent.status == expected {
            Ok(intent)
        } else {
            Err(KernelError::IntentStatusMismatch)
        }
    }

    pub(crate) fn set_intent_status(
        &mut self,
        id: IntentId,
        status: IntentStatus,
    ) -> Result<(), KernelError> {
        self.find_intent_mut(id)?.status = status;
        Ok(())
    }

    fn record_intent_event(
        &mut self,
        agent: AgentId,
        capability: CapabilityId,
        resource: ResourceId,
        intent: IntentId,
        kind: IntentKind,
        verification: VerificationRequirement,
    ) -> Result<Event, KernelError> {
        self.record(Event {
            sequence: self.next_sequence,
            agent,
            kind: EventKind::IntentDeclared,
            resource: Some(resource),
            capability: Some(capability),
            source_capability: None,
            intent: Some(intent),
            intent_kind: Some(kind),
            action: None,
            observation: None,
            message: None,
            memory_cell: None,
            operation: Some(kind.required_operation()),
            operations: OperationSet::empty(),
            verification,
            checkpoint: None,
            task: None,
            target_agent: None,
        })
    }
}
