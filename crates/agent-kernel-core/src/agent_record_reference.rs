//! Complete non-Event reference preflight for Agent Record retirement.
//!
//! This no_std Core child centralizes every fixed Store field that can retain
//! an Agent identity. Historical Events are deliberately excluded because the
//! retirement high-water prevents identity aliasing during later registration.

use crate::{AgentId, KernelCore, KernelError, NamespaceObject};

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
    pub(crate) fn ensure_agent_record_unreferenced(
        &self,
        target: AgentId,
    ) -> Result<(), KernelError> {
        let referenced = self.agents[..self.agent_len]
            .iter()
            .any(|record| record.id != target && record.manager == Some(target))
            || self.resources[..self.resource_len]
                .iter()
                .any(|resource| resource.owner == Some(target))
            || self
                .capabilities
                .iter()
                .flatten()
                .any(|capability| capability.agent == target)
            || self.intents[..self.intent_len]
                .iter()
                .any(|intent| intent.owner == target)
            || self.tasks[..self.task_len]
                .iter()
                .any(|task| task.owner == target || task.assignee == Some(target))
            || self.run_queue[..self.run_queue_len]
                .iter()
                .any(|entry| entry.agent == target)
            || self.runtime_admissions[..self.runtime_admission_len]
                .iter()
                .any(|record| record.requester == target || record.target == target)
            || self.actions[..self.action_len]
                .iter()
                .any(|record| record.agent == target)
            || self.observations[..self.observation_len]
                .iter()
                .any(|record| record.agent == target)
            || self.checkpoints[..self.checkpoint_len]
                .iter()
                .any(|record| record.agent == target)
            || self.messages[..self.message_len]
                .iter()
                .any(|record| record.sender == target || record.recipient == target)
            || self.memory_cells[..self.memory_cell_len]
                .iter()
                .any(|record| record.creator == target || record.last_writer == target)
            || self.namespace_entries[..self.namespace_entry_len]
                .iter()
                .any(|record| {
                    record.owner == target || record.object == NamespaceObject::Agent(target)
                })
            || self.faults[..self.fault_len]
                .iter()
                .any(|record| record.agent == target)
            || self.fault_handlers[..self.fault_handler_len]
                .iter()
                .any(|record| record.installer == target || record.handler == target)
            || self.fault_policies[..self.fault_policy_len]
                .iter()
                .any(|record| record.installer == target)
            || self.waiters[..self.waiter_len]
                .iter()
                .any(|record| record.agent == target)
            || self.agent_images[..self.agent_image_len]
                .iter()
                .any(|record| record.owner == target)
            || self.agent_entries[..self.agent_entry_len]
                .iter()
                .any(|record| record.agent == target)
            || self.driver_endpoints[..self.driver_endpoint_len]
                .iter()
                .any(|record| record.installer == target)
            || self.driver_bindings[..self.driver_binding_len]
                .iter()
                .any(|record| record.installer == target || record.driver == target)
            || self.driver_commands[..self.driver_command_len]
                .iter()
                .any(|record| record.driver == target)
            || self.driver_invocations[..self.driver_invocation_len]
                .iter()
                .any(|record| record.driver == target);

        if referenced {
            Err(KernelError::AgentRecordRetirementReferenced)
        } else {
            Ok(())
        }
    }
}
