//! Capability authorization checks.
//!
//! This module owns the invariant that resource operations require a matching,
//! non-revoked capability chain for the agent, resource, and operation.

use crate::{
    AgentId, Capability, CapabilityId, KernelCore, KernelError, Operation, ResourceId, TaskId,
};

impl<
        const RESOURCES: usize,
        const CAPS: usize,
        const EVENTS: usize,
        const TASKS: usize,
        const RUN_QUEUE: usize,
    > KernelCore<RESOURCES, CAPS, EVENTS, TASKS, RUN_QUEUE>
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

    fn ensure_capability_chain_active(&self, capability: Capability) -> Result<(), KernelError> {
        let mut current = capability;

        for _ in 0..CAPS {
            if current.revoked {
                return Err(KernelError::CapabilityRevoked);
            }

            let Some(parent) = current.parent else {
                return Ok(());
            };
            current = self.find_capability(parent)?;
        }

        Err(KernelError::CapabilityNotFound)
    }
}
