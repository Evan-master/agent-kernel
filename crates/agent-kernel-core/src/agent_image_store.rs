//! Fixed-capacity Agent Image store.
//!
//! This core-layer module owns image registration and lookup. It stores
//! executable identity metadata only and keeps every successful mutation
//! replayable through explicit image events.

use crate::{
    AgentEntryKind, AgentId, AgentImageDigest, AgentImageId, AgentImageKind, AgentImageRecord,
    AgentImageStatus, CapabilityId, Event, KernelCore, KernelError, Operation, ResourceId,
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
    >
{
    pub fn register_agent_image(
        &mut self,
        owner: AgentId,
        capability: CapabilityId,
        resource: ResourceId,
        kind: AgentImageKind,
        digest: AgentImageDigest,
        abi_version: u16,
        entry_version: u16,
    ) -> Result<AgentImageId, KernelError> {
        self.ensure_agent_active(owner)?;
        self.ensure_authorized(owner, capability, resource, Operation::Act)?;
        if abi_version == 0 || entry_version == 0 {
            return Err(KernelError::AgentImageVersionInvalid);
        }
        if self.agent_image_len >= AGENT_IMAGES {
            return Err(KernelError::AgentImageStoreFull);
        }
        self.ensure_event_slots(1)?;

        let image = AgentImageId::new(self.next_agent_image);
        self.next_agent_image += 1;
        self.agent_images[self.agent_image_len] = AgentImageRecord {
            id: image,
            owner,
            resource,
            kind,
            digest,
            abi_version,
            entry_version,
            status: AgentImageStatus::Pending,
        };
        self.agent_image_len += 1;
        self.record_agent_image_registered_event(
            owner,
            capability,
            resource,
            image,
            kind,
            digest,
            abi_version,
            entry_version,
        )?;
        Ok(image)
    }

    pub fn agent_images(&self) -> &[AgentImageRecord] {
        &self.agent_images[..self.agent_image_len]
    }

    pub fn agent_image(&self, image: AgentImageId) -> Result<AgentImageRecord, KernelError> {
        self.find_agent_image(image)
    }

    pub(crate) fn find_agent_image(
        &self,
        image: AgentImageId,
    ) -> Result<AgentImageRecord, KernelError> {
        self.agent_images()
            .iter()
            .find(|record| record.id == image)
            .copied()
            .ok_or(KernelError::AgentImageNotFound)
    }

    pub(crate) fn find_agent_image_mut(
        &mut self,
        image: AgentImageId,
    ) -> Result<&mut AgentImageRecord, KernelError> {
        self.agent_images[..self.agent_image_len]
            .iter_mut()
            .find(|record| record.id == image)
            .ok_or(KernelError::AgentImageNotFound)
    }

    pub(crate) fn ensure_launch_image(
        &self,
        image: AgentImageId,
        resource: ResourceId,
        entry_kind: AgentEntryKind,
    ) -> Result<AgentImageRecord, KernelError> {
        let record = self.find_agent_image(image)?;
        match record.status {
            AgentImageStatus::Verified => {}
            AgentImageStatus::Pending => return Err(KernelError::AgentImageStatusMismatch),
            AgentImageStatus::Retired => return Err(KernelError::AgentImageRetired),
        }
        if record.resource != resource {
            return Err(KernelError::AgentImageResourceMismatch);
        }
        if !image_kind_matches_entry(record.kind, entry_kind) {
            return Err(KernelError::AgentImageKindMismatch);
        }
        Ok(record)
    }

    pub fn verify_agent_image(
        &mut self,
        owner: AgentId,
        capability: CapabilityId,
        image: AgentImageId,
    ) -> Result<Event, KernelError> {
        self.ensure_agent_active(owner)?;
        let record = self.find_agent_image(image)?;
        if record.owner != owner {
            return Err(KernelError::AgentMismatch);
        }
        match record.status {
            AgentImageStatus::Pending => {}
            AgentImageStatus::Verified => return Err(KernelError::AgentImageStatusMismatch),
            AgentImageStatus::Retired => return Err(KernelError::AgentImageRetired),
        }
        self.ensure_authorized(owner, capability, record.resource, Operation::Verify)?;
        self.ensure_event_slots(1)?;

        self.find_agent_image_mut(image)?.status = AgentImageStatus::Verified;
        self.record_agent_image_verified_event(
            owner,
            capability,
            record.resource,
            image,
            record.kind,
        )
    }

    pub fn retire_agent_image(
        &mut self,
        owner: AgentId,
        capability: CapabilityId,
        image: AgentImageId,
    ) -> Result<Event, KernelError> {
        self.ensure_agent_active(owner)?;
        let record = self.find_agent_image(image)?;
        match record.status {
            AgentImageStatus::Pending | AgentImageStatus::Verified => {}
            AgentImageStatus::Retired => return Err(KernelError::AgentImageRetired),
        }
        if record.owner != owner {
            return Err(KernelError::AgentMismatch);
        }
        self.ensure_authorized(owner, capability, record.resource, Operation::Rollback)?;
        self.ensure_event_slots(1)?;

        self.find_agent_image_mut(image)?.status = AgentImageStatus::Retired;
        self.record_agent_image_retired_event(
            owner,
            capability,
            record.resource,
            image,
            record.kind,
        )
    }
}

fn image_kind_matches_entry(image: AgentImageKind, entry: AgentEntryKind) -> bool {
    matches!(
        (image, entry),
        (AgentImageKind::Bootstrap, AgentEntryKind::Bootstrap)
            | (AgentImageKind::Supervisor, AgentEntryKind::Supervisor)
            | (AgentImageKind::Worker, AgentEntryKind::Worker)
    )
}
