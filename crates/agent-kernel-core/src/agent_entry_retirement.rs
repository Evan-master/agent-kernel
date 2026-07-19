//! Authenticated retirement of one quiescent Agent launch entry.
//!
//! This no_std core module validates terminal scope, execution quiescence,
//! cleanup authority, and live kernel references before removing one record
//! from the dense fixed-capacity Entry Store. Every mutation emits one Event.

use crate::{
    AgentEntryKind, AgentEntryRecord, AgentEntryRetirement, AgentExecutionState, AgentId,
    AgentStatus, CapabilityId, Event, EventKind, IntentStatus, KernelCore, KernelError,
    MessageStatus, Operation, RuntimeAdmissionStatus, TaskStatus,
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
    pub fn retire_agent_entry(
        &mut self,
        actor: AgentId,
        authority: CapabilityId,
        target: AgentId,
    ) -> Result<AgentEntryRetirement, KernelError> {
        let actor_entry = self
            .find_agent_entry(actor)
            .map_err(|_| KernelError::AgentNotLaunched)?;
        if actor_entry.kind != AgentEntryKind::Supervisor {
            return Err(KernelError::AgentEntryKindMismatch);
        }

        let index = self
            .agent_entries()
            .iter()
            .position(|entry| entry.agent == target)
            .ok_or(KernelError::AgentEntryNotFound)?;
        let entry = self.agent_entries[index];
        self.ensure_agent_entry_retirement_ready(entry)?;
        self.ensure_cleanup_authorized(actor, authority, entry.resource)?;
        self.ensure_agent_entry_unreferenced(target)?;
        self.ensure_event_slots(1)?;

        let previous = self.agent_entries;
        let remaining = self.agent_entry_len - 1;
        self.agent_entries[index..remaining]
            .copy_from_slice(&previous[index + 1..self.agent_entry_len]);
        self.agent_entries[remaining] = AgentEntryRecord::empty();
        self.agent_entry_len = remaining;
        self.record(agent_entry_retirement_event(entry, actor, authority))?;

        Ok(AgentEntryRetirement::new(entry))
    }

    fn ensure_agent_entry_retirement_ready(
        &self,
        entry: AgentEntryRecord,
    ) -> Result<(), KernelError> {
        let context = self.execution_context(entry.agent)?;
        if context.state != AgentExecutionState::Idle
            || context.task.is_some()
            || context.driver_invocation.is_some()
        {
            return Err(KernelError::AgentEntryRetirementNotReady);
        }

        if let Some(task) = entry.task {
            return match self.task(task) {
                Ok(task)
                    if task.assignee == Some(entry.agent)
                        && task.resource == entry.resource
                        && task.delegated_capability == Some(entry.capability)
                        && matches!(
                            task.status,
                            TaskStatus::Completed | TaskStatus::Verified | TaskStatus::Cancelled
                        ) =>
                {
                    Ok(())
                }
                Err(KernelError::TaskNotFound) => Ok(()),
                _ => Err(KernelError::AgentEntryRetirementNotReady),
            };
        }

        if let Some(intent) = entry.intent {
            return match self.intent(intent) {
                Ok(intent)
                    if intent.owner == entry.agent
                        && intent.resource == entry.resource
                        && matches!(
                            intent.status,
                            IntentStatus::Fulfilled | IntentStatus::Cancelled
                        ) =>
                {
                    Ok(())
                }
                Err(KernelError::IntentNotFound) => Ok(()),
                _ => Err(KernelError::AgentEntryRetirementNotReady),
            };
        }

        if self.find_agent(entry.agent)?.status == AgentStatus::Retired {
            Ok(())
        } else {
            Err(KernelError::AgentEntryRetirementNotReady)
        }
    }

    fn ensure_agent_entry_unreferenced(&self, target: AgentId) -> Result<(), KernelError> {
        let referenced = self.run_queue().iter().any(|entry| entry.agent == target)
            || self
                .waiters()
                .iter()
                .any(|waiter| waiter.active && waiter.agent == target)
            || self.runtime_admissions().iter().any(|admission| {
                matches!(
                    admission.status,
                    RuntimeAdmissionStatus::Requested | RuntimeAdmissionStatus::Admitted
                ) && (admission.requester == target || admission.target == target)
            })
            || self.messages().iter().any(|message| {
                message.recipient == target && message.status == MessageStatus::Received
            })
            || self
                .fault_handlers()
                .iter()
                .any(|handler| handler.handler == target)
            || self
                .driver_bindings()
                .iter()
                .any(|binding| binding.driver == target);

        if referenced {
            Err(KernelError::AgentEntryRetirementReferenced)
        } else {
            Ok(())
        }
    }
}

fn agent_entry_retirement_event(
    entry: AgentEntryRecord,
    actor: AgentId,
    authority: CapabilityId,
) -> Event {
    let mut event = Event::empty();
    event.agent = actor;
    event.kind = EventKind::AgentEntryRetired;
    event.resource = Some(entry.resource);
    event.capability = Some(entry.capability);
    event.source_capability = Some(authority);
    event.intent = entry.intent;
    event.operation = Some(Operation::Rollback);
    event.task = entry.task;
    event.target_agent = Some(entry.agent);
    event.agent_image = Some(entry.image);
    event.agent_image_kind = Some(entry.kind.image_kind());
    event
}
