//! Shared authorization for terminal metadata cleanup.
//!
//! This no_std core module accepts exact `Rollback` authority for active
//! Resources and active-ancestor authority for retired descendants. Resource
//! ancestry is immutable and every walk is bounded by fixed store capacity.

use crate::{
    AgentId, CapabilityId, KernelCore, KernelError, Operation, ResourceId, ResourceStatus,
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
    pub(crate) fn ensure_cleanup_authorized(
        &self,
        actor: AgentId,
        authority: CapabilityId,
        target_resource: ResourceId,
    ) -> Result<(), KernelError> {
        let authority_record = self.find_capability(authority)?;
        self.ensure_authorized(
            actor,
            authority,
            authority_record.resource,
            Operation::Rollback,
        )?;

        let target_record = self
            .resources()
            .iter()
            .find(|resource| resource.id == target_resource)
            .copied()
            .ok_or(KernelError::ResourceNotFound)?;
        if target_record.status == ResourceStatus::Active {
            return if authority_record.resource == target_resource {
                Ok(())
            } else {
                Err(KernelError::ResourceMismatch)
            };
        }

        let mut current = target_record;
        for _ in 0..RESOURCES {
            let Some(parent) = current.parent else {
                return Err(KernelError::ResourceMismatch);
            };
            if parent == authority_record.resource {
                return Ok(());
            }
            current = self
                .resources()
                .iter()
                .find(|resource| resource.id == parent)
                .copied()
                .ok_or(KernelError::ResourceNotFound)?;
        }

        Err(KernelError::ResourceMismatch)
    }
}
