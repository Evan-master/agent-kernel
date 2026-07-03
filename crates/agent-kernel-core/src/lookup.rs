//! Fixed-capacity store lookup helpers.
//!
//! This module owns read and mutable lookup behavior for resources and
//! capabilities. It is separated from the core state machine to keep public
//! lifecycle methods easier to scan.

use crate::{Capability, CapabilityId, KernelCore, KernelError, Resource, ResourceId};

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
    >
{
    pub(crate) fn find_resource(&self, id: ResourceId) -> Result<Resource, KernelError> {
        self.resources
            .iter()
            .flatten()
            .find(|resource| resource.id == id)
            .copied()
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
