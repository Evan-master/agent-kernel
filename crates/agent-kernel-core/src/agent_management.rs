//! Capability-authorized Agent identity management.
//!
//! Registers managed Agent identities under root-scoped `Delegate` authority.
//! Targets stay unlaunched, idle, and free of active Tasks before status changes.

use crate::{
    AgentExecutionState, AgentId, AgentRecord, AgentStatus, CapabilityId, Event, EventKind,
    KernelCore, KernelError, Operation, ResourceId, TaskStatus,
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
    pub fn register_managed_agent(
        &mut self,
        manager: AgentId,
        capability: CapabilityId,
        resource: ResourceId,
        target: AgentId,
    ) -> Result<Event, KernelError> {
        self.ensure_management_identities(manager, target)?;
        self.ensure_authorized(manager, capability, resource, Operation::Delegate)?;
        self.ensure_agent_registration_available(target)?;
        self.ensure_event_slots(1)?;
        self.insert_agent_record(target, Some(manager), Some(resource));
        self.record_agent_event(
            EventKind::AgentRegistered,
            manager,
            target,
            Some(resource),
            Some(capability),
            Some(Operation::Delegate),
        )
    }

    pub fn suspend_managed_agent(
        &mut self,
        actor: AgentId,
        capability: CapabilityId,
        target: AgentId,
    ) -> Result<Event, KernelError> {
        let (record, resource) = self.authorize_managed_transition(actor, capability, target)?;
        match record.status {
            AgentStatus::Active => self.transition_managed_agent(
                actor,
                capability,
                target,
                resource,
                AgentStatus::Suspended,
                EventKind::AgentSuspended,
            ),
            AgentStatus::Suspended => Err(KernelError::AgentStatusMismatch),
            AgentStatus::Retired => Err(KernelError::AgentRetired),
        }
    }

    pub fn resume_managed_agent(
        &mut self,
        actor: AgentId,
        capability: CapabilityId,
        target: AgentId,
    ) -> Result<Event, KernelError> {
        let (record, resource) = self.authorize_managed_transition(actor, capability, target)?;
        match record.status {
            AgentStatus::Suspended => self.transition_managed_agent(
                actor,
                capability,
                target,
                resource,
                AgentStatus::Active,
                EventKind::AgentResumed,
            ),
            AgentStatus::Active => Err(KernelError::AgentStatusMismatch),
            AgentStatus::Retired => Err(KernelError::AgentRetired),
        }
    }

    pub fn retire_managed_agent(
        &mut self,
        actor: AgentId,
        capability: CapabilityId,
        target: AgentId,
    ) -> Result<Event, KernelError> {
        let (record, resource) = self.authorize_managed_transition(actor, capability, target)?;
        match record.status {
            AgentStatus::Active | AgentStatus::Suspended => self.transition_managed_agent(
                actor,
                capability,
                target,
                resource,
                AgentStatus::Retired,
                EventKind::AgentRetired,
            ),
            AgentStatus::Retired => Err(KernelError::AgentRetired),
        }
    }

    fn authorize_managed_transition(
        &self,
        actor: AgentId,
        capability: CapabilityId,
        target: AgentId,
    ) -> Result<(AgentRecord, ResourceId), KernelError> {
        self.ensure_management_identities(actor, target)?;
        let record = self.find_agent(target)?;
        let Some(resource) = record.management_resource else {
            return Err(KernelError::AgentManagementDenied);
        };
        if record.manager.is_none() {
            return Err(KernelError::AgentManagementDenied);
        }
        self.ensure_authorized(actor, capability, resource, Operation::Delegate)?;
        self.ensure_managed_target_quiescent(target)?;
        Ok((record, resource))
    }

    fn ensure_management_identities(
        &self,
        actor: AgentId,
        target: AgentId,
    ) -> Result<(), KernelError> {
        if actor.raw() == 0 || target.raw() == 0 {
            return Err(KernelError::AgentManagementDenied);
        }
        if actor == target {
            return Err(KernelError::AgentSelfManagementDenied);
        }
        Ok(())
    }

    fn ensure_managed_target_quiescent(&self, target: AgentId) -> Result<(), KernelError> {
        let context = self.execution_context(target)?;
        let context_busy = context.state != AgentExecutionState::Idle
            || context.task.is_some()
            || context.driver_invocation.is_some();
        let launched = self
            .agent_entries()
            .iter()
            .any(|entry| entry.agent == target);
        let assigned_task = self.tasks().iter().any(|task| {
            task.assignee == Some(target)
                && !matches!(
                    task.status,
                    TaskStatus::Completed | TaskStatus::Verified | TaskStatus::Cancelled
                )
        });
        if context_busy || launched || assigned_task {
            Err(KernelError::AgentManagementBusy)
        } else {
            Ok(())
        }
    }

    fn transition_managed_agent(
        &mut self,
        actor: AgentId,
        capability: CapabilityId,
        target: AgentId,
        resource: ResourceId,
        status: AgentStatus,
        kind: EventKind,
    ) -> Result<Event, KernelError> {
        self.ensure_event_slots(1)?;
        self.find_agent_mut(target)?.status = status;
        self.record_agent_event(
            kind,
            actor,
            target,
            Some(resource),
            Some(capability),
            Some(Operation::Delegate),
        )
    }
}
