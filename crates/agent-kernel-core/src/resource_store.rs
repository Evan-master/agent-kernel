//! Fixed-capacity resource registration.
//!
//! This module belongs to `agent-kernel-core`. It owns deterministic resource
//! allocation for the no_std core store. It performs no host I/O and validates
//! parent resources before adding child resources.

use crate::{KernelCore, KernelError, Resource, ResourceId, ResourceKind};

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
    >
{
    pub fn register_resource(
        &mut self,
        kind: ResourceKind,
        parent: Option<ResourceId>,
    ) -> Result<ResourceId, KernelError> {
        if let Some(parent_id) = parent {
            self.find_resource(parent_id)?;
        }

        let slot = self
            .resources
            .iter_mut()
            .find(|resource| resource.is_none())
            .ok_or(KernelError::ResourceStoreFull)?;
        let id = ResourceId::new(self.next_resource);
        self.next_resource += 1;
        *slot = Some(Resource { id, kind, parent });
        Ok(id)
    }
}
