//! Capability authorization checks.
//!
//! This module owns the invariant that resource operations require a matching,
//! non-revoked capability for the agent, resource, and operation.

use crate::{AgentId, CapabilityId, KernelCore, KernelError, Operation, ResourceId};

impl<const RESOURCES: usize, const CAPS: usize, const EVENTS: usize, const TASKS: usize>
    KernelCore<RESOURCES, CAPS, EVENTS, TASKS>
{
    pub(crate) fn ensure_authorized(
        &self,
        agent: AgentId,
        capability: CapabilityId,
        resource: ResourceId,
        operation: Operation,
    ) -> Result<(), KernelError> {
        self.find_resource(resource)?;
        let cap = self.find_capability(capability)?;

        if cap.revoked {
            return Err(KernelError::CapabilityRevoked);
        }
        if cap.agent != agent {
            return Err(KernelError::AgentMismatch);
        }
        if cap.resource != resource {
            return Err(KernelError::ResourceMismatch);
        }
        if !cap.operations.allows(operation) {
            return Err(KernelError::OperationDenied);
        }

        Ok(())
    }
}
