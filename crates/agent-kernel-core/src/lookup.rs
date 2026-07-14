//! Fixed-capacity store lookup helpers.
//!
//! This module owns read and mutable lookup behavior for resources and
//! capabilities. It is separated from the core state machine to keep public
//! lifecycle methods easier to scan.

use crate::{
    Capability, CapabilityId, KernelCore, KernelError, Resource, ResourceId, ResourceStatus,
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
    >
{
    pub(crate) fn find_resource(&self, id: ResourceId) -> Result<Resource, KernelError> {
        let resource = self
            .resources()
            .iter()
            .find(|resource| resource.id == id)
            .copied()
            .ok_or(KernelError::ResourceNotFound)?;
        if resource.status == ResourceStatus::Retired {
            return Err(KernelError::ResourceRetired);
        }
        Ok(resource)
    }

    pub(crate) fn find_resource_mut(
        &mut self,
        id: ResourceId,
    ) -> Result<&mut Resource, KernelError> {
        self.resources[..self.resource_len]
            .iter_mut()
            .find(|resource| resource.id == id)
            .ok_or(KernelError::ResourceNotFound)
    }

    pub(crate) fn find_capability(&self, id: CapabilityId) -> Result<Capability, KernelError> {
        self.capabilities
            .iter()
            .flatten()
            .find(|capability| capability.id == id)
            .copied()
            .ok_or(KernelError::CapabilityNotFound)
    }

    pub(crate) fn find_capability_mut(
        &mut self,
        id: CapabilityId,
    ) -> Result<&mut Capability, KernelError> {
        self.capabilities
            .iter_mut()
            .flatten()
            .find(|capability| capability.id == id)
            .ok_or(KernelError::CapabilityNotFound)
    }
}
