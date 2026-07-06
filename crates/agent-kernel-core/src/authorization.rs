//! Capability authorization checks.
//!
//! This module owns the invariant that resource operations require a registered
//! actor and a matching, non-revoked capability chain for the resource and
//! operation.

use crate::{
    AgentId, Capability, CapabilityId, KernelCore, KernelError, Operation, ResourceId, TaskId,
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
    >
{
    pub(crate) fn ensure_authorized(
        &self,
        agent: AgentId,
        capability: CapabilityId,
        resource: ResourceId,
        operation: Operation,
    ) -> Result<(), KernelError> {
        let cap = self.ensure_capability_base(agent, capability, resource, operation)?;
        if cap.task.is_some() {
            return Err(KernelError::CapabilityScopeMismatch);
        }

        Ok(())
    }

    pub(crate) fn ensure_authorized_for_task(
        &self,
        agent: AgentId,
        capability: CapabilityId,
        resource: ResourceId,
        operation: Operation,
        task: TaskId,
    ) -> Result<(), KernelError> {
        let cap = self.ensure_capability_base(agent, capability, resource, operation)?;
        if let Some(scope) = cap.task {
            if scope != task {
                return Err(KernelError::CapabilityScopeMismatch);
            }
        }

        Ok(())
    }

    fn ensure_capability_base(
        &self,
        agent: AgentId,
        capability: CapabilityId,
        resource: ResourceId,
        operation: Operation,
    ) -> Result<Capability, KernelError> {
        self.ensure_agent_active(agent)?;
        self.find_resource(resource)?;
        let cap = self.find_capability(capability)?;

        self.ensure_capability_chain_active(cap)?;
        if cap.agent != agent {
            return Err(KernelError::AgentMismatch);
        }
        if cap.resource != resource {
            return Err(KernelError::ResourceMismatch);
        }
        if !cap.operations.allows(operation) {
            return Err(KernelError::OperationDenied);
        }

        Ok(cap)
    }

    pub(crate) fn ensure_capability_chain_active(
        &self,
        capability: Capability,
    ) -> Result<(), KernelError> {
        let mut current = capability;

        for _ in 0..CAPS {
            if current.revoked {
                return Err(KernelError::CapabilityRevoked);
            }
            self.ensure_agent_active(current.agent)?;

            let Some(parent) = current.parent else {
                return Ok(());
            };
            current = self.find_capability(parent)?;
        }

        Err(KernelError::CapabilityNotFound)
    }
}
