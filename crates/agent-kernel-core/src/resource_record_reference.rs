//! Complete non-Event reference preflight for Resource record retirement.
//!
//! This no_std Core child scans every fixed Store field that can retain a
//! Resource identity. Events remain historical references because Resource
//! allocation is monotonic and cannot alias a retired ID.

use crate::{KernelCore, KernelError, NamespaceObject, ResourceId};

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
    pub(crate) fn ensure_resource_record_unreferenced(
        &self,
        target: ResourceId,
    ) -> Result<(), KernelError> {
        let referenced = self.resources[..self.resource_len]
            .iter()
            .any(|record| record.id != target && record.parent == Some(target))
            || self
                .capabilities
                .iter()
                .flatten()
                .any(|record| record.resource == target)
            || self.agents[..self.agent_len]
                .iter()
                .any(|record| record.management_resource == Some(target))
            || self.agent_entries[..self.agent_entry_len]
                .iter()
                .any(|record| record.resource == target)
            || self.agent_images[..self.agent_image_len]
                .iter()
                .any(|record| record.resource == target)
            || self.intents[..self.intent_len]
                .iter()
                .any(|record| record.resource == target)
            || self.tasks[..self.task_len]
                .iter()
                .any(|record| record.resource == target)
            || self.runtime_admissions[..self.runtime_admission_len]
                .iter()
                .any(|record| record.resource == target)
            || self.actions[..self.action_len]
                .iter()
                .any(|record| record.resource == target)
            || self.observations[..self.observation_len]
                .iter()
                .any(|record| record.resource == target)
            || self.checkpoints[..self.checkpoint_len]
                .iter()
                .any(|record| record.resource == target)
            || self.messages[..self.message_len]
                .iter()
                .any(|record| record.payload.resource == Some(target))
            || self.memory_cells[..self.memory_cell_len]
                .iter()
                .any(|record| record.resource == target)
            || self.namespace_entries[..self.namespace_entry_len]
                .iter()
                .any(|record| {
                    record.namespace == target
                        || record.object == NamespaceObject::Resource(target)
                        || record.object == NamespaceObject::Mount(target)
                })
            || self.faults[..self.fault_len]
                .iter()
                .any(|record| record.resource == target)
            || self.fault_handlers[..self.fault_handler_len]
                .iter()
                .any(|record| record.resource == target)
            || self.fault_policies[..self.fault_policy_len]
                .iter()
                .any(|record| record.resource == target)
            || self.waiters[..self.waiter_len]
                .iter()
                .any(|record| record.resource == target)
            || self.driver_endpoints[..self.driver_endpoint_len]
                .iter()
                .any(|record| record.resource == target)
            || self.driver_bindings[..self.driver_binding_len]
                .iter()
                .any(|record| record.resource == target)
            || self.device_events[..self.device_event_len]
                .iter()
                .any(|record| record.resource == target)
            || self.driver_commands[..self.driver_command_len]
                .iter()
                .any(|record| record.resource == target)
            || self.driver_invocations[..self.driver_invocation_len]
                .iter()
                .any(|record| record.resource == target)
            || self
                .event_archive_checkpoint
                .is_some_and(|checkpoint| checkpoint.root() == target);

        if referenced {
            Err(KernelError::ResourceRecordRetirementReferenced)
        } else {
            Ok(())
        }
    }
}
