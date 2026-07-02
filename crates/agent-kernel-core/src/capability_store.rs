//! Fixed-capacity capability grants and revocation.
//!
//! This module belongs to `agent-kernel-core`. It owns deterministic capability
//! allocation and revocation while preserving the invariant that all grants
//! point at an existing resource.

use crate::{
    AgentId, Capability, CapabilityId, KernelCore, KernelError, OperationSet, ResourceId, TaskId,
};

impl<
        const RESOURCES: usize,
        const CAPS: usize,
        const EVENTS: usize,
        const TASKS: usize,
        const RUN_QUEUE: usize,
    > KernelCore<RESOURCES, CAPS, EVENTS, TASKS, RUN_QUEUE>
{
    pub fn grant_capability(
        &mut self,
        agent: AgentId,
        resource: ResourceId,
        operations: OperationSet,
    ) -> Result<CapabilityId, KernelError> {
        self.find_resource(resource)?;

        let slot = self
            .capabilities
            .iter_mut()
            .find(|capability| capability.is_none())
            .ok_or(KernelError::CapabilityStoreFull)?;
        let id = CapabilityId::new(self.next_capability);
        self.next_capability += 1;
        *slot = Some(Capability {
            id,
            agent,
            resource,
            operations,
            revoked: false,
            task: None,
            parent: None,
        });
        Ok(id)
    }

    pub(crate) fn derive_task_capability(
        &mut self,
        agent: AgentId,
        resource: ResourceId,
        operations: OperationSet,
        task: TaskId,
        parent: CapabilityId,
    ) -> Result<CapabilityId, KernelError> {
        self.find_resource(resource)?;
        self.find_capability(parent)?;

        let slot = self
            .capabilities
            .iter_mut()
            .find(|capability| capability.is_none())
            .ok_or(KernelError::CapabilityStoreFull)?;
        let id = CapabilityId::new(self.next_capability);
        self.next_capability += 1;
        *slot = Some(Capability {
            id,
            agent,
            resource,
            operations,
            revoked: false,
            task: Some(task),
            parent: Some(parent),
        });
        Ok(id)
    }

    pub fn revoke_capability(&mut self, capability: CapabilityId) -> Result<(), KernelError> {
        let cap = self.find_capability_mut(capability)?;
        cap.revoked = true;
        Ok(())
    }
}
