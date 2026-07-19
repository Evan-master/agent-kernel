//! Authenticated retirement of a terminal Intent prefix.
//!
//! This no_std core module validates Supervisor authority, terminal state,
//! active Task and Message references, and Event capacity before reclaiming
//! fixed Intent slots. Intent IDs remain monotonic and historical references
//! stay available through ordered Events and launch records.

use crate::intent_event::intent_compaction_event;
use crate::{
    AgentEntryKind, AgentId, CapabilityId, Intent, IntentCompaction, IntentId, IntentStatus,
    KernelCore, KernelError, MessageStatus, Operation,
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
        const RUNTIME_ADMISSIONS: usize,
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
        RUNTIME_ADMISSIONS,
    >
{
    pub fn compact_intent_prefix(
        &mut self,
        actor: AgentId,
        authority: CapabilityId,
        through: IntentId,
    ) -> Result<IntentCompaction, KernelError> {
        let actor_entry = self
            .find_agent_entry(actor)
            .map_err(|_| KernelError::AgentNotLaunched)?;
        if actor_entry.kind != AgentEntryKind::Supervisor {
            return Err(KernelError::AgentEntryKindMismatch);
        }

        let through_index = self
            .intents()
            .iter()
            .position(|intent| intent.id == through)
            .ok_or(KernelError::IntentNotFound)?;
        let count = through_index + 1;
        for intent in self.intents()[..count].iter().copied() {
            self.ensure_intent_compaction_ready(intent)?;
            self.ensure_authorized(actor, authority, intent.resource, Operation::Rollback)?;
        }
        self.ensure_event_slots(count)?;

        let previous = self.intents;
        let remaining = self.intent_len - count;
        self.intents[..remaining].copy_from_slice(&previous[count..self.intent_len]);
        for index in remaining..self.intent_len {
            self.intents[index] = Intent::empty();
        }
        self.intent_len = remaining;
        for intent in previous[..count].iter().copied() {
            self.record(intent_compaction_event(intent, actor, authority))?;
        }

        Ok(IntentCompaction::new(previous[0].id, through, count))
    }

    fn ensure_intent_compaction_ready(&self, intent: Intent) -> Result<(), KernelError> {
        if !matches!(
            intent.status,
            IntentStatus::Fulfilled | IntentStatus::Cancelled
        ) {
            return Err(KernelError::IntentCompactionNotReady);
        }

        let referenced = self.tasks().iter().any(|task| task.intent == intent.id)
            || self.messages().iter().any(|message| {
                message.payload.intent == Some(intent.id)
                    && message.status != MessageStatus::Acknowledged
            });
        if referenced {
            Err(KernelError::IntentCompactionReferenced)
        } else {
            Ok(())
        }
    }
}
