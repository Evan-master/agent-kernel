//! Capability-authorized retirement of one terminal managed Agent Record.
//!
//! This no_std Core module validates lifecycle, execution quiescence, exact
//! Delegate authority, complete Store liveness, and Event capacity before it
//! atomically removes the index-aligned Agent and execution-context records.

use crate::{
    AgentExecutionState, AgentId, AgentRecordRetirement, AgentStatus, CapabilityId, EventKind,
    KernelCore, KernelError, Operation,
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
    pub fn retire_agent_record(
        &mut self,
        actor: AgentId,
        authority: CapabilityId,
        target: AgentId,
    ) -> Result<AgentRecordRetirement, KernelError> {
        self.ensure_agent_active(actor)?;
        let index = self
            .agents()
            .iter()
            .position(|record| record.id == target)
            .ok_or(KernelError::AgentNotFound)?;
        let record = self.agents[index];
        if record.status != AgentStatus::Retired {
            return Err(KernelError::AgentRecordRetirementNotReady);
        }
        let Some(management_resource) = record.management_resource else {
            return Err(KernelError::AgentManagementDenied);
        };
        if record.manager.is_none() {
            return Err(KernelError::AgentManagementDenied);
        }
        self.ensure_authorized(actor, authority, management_resource, Operation::Delegate)?;

        let context = self.execution_contexts[index];
        if context.agent != target
            || context.state != AgentExecutionState::Idle
            || context.task.is_some()
            || context.driver_invocation.is_some()
        {
            return Err(KernelError::AgentRecordRetirementNotReady);
        }
        self.ensure_agent_record_unreferenced(target)?;
        self.ensure_event_slots(1)?;

        let (record, context) = self.remove_agent_record_at(index);
        let retired_floor = self.retired_agent_floor();
        self.record_agent_event(
            EventKind::AgentRecordRetired,
            actor,
            target,
            Some(management_resource),
            Some(authority),
            Some(Operation::Delegate),
        )?;
        Ok(AgentRecordRetirement::new(
            record,
            context,
            actor,
            authority,
            management_resource,
            retired_floor,
        ))
    }
}
