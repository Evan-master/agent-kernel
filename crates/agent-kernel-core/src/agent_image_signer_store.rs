//! Fixed-capacity runtime Trust Policy state machine.
//!
//! This core-layer module owns capability-authorized initial trust and atomic
//! signer rotation. Every check is completed before records, generations, or
//! Events change, preserving deterministic zero-side-effect failures.

use crate::{
    agent_image_signer_id, AgentId, AgentImageKindScope, AgentImageSignerId,
    AgentImageSignerRecord, AgentImageSignerRotation, AgentImageSignerStatus, CapabilityId,
    EventKind, KernelCore, KernelError, Operation, ResourceId,
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
    pub fn trust_agent_image_signer(
        &mut self,
        actor: AgentId,
        authority: CapabilityId,
        resource: ResourceId,
        public_key: [u8; 32],
        image_kinds: AgentImageKindScope,
        minimum_abi: u16,
        maximum_abi: u16,
    ) -> Result<AgentImageSignerRecord, KernelError> {
        self.ensure_agent_active(actor)?;
        self.ensure_authorized(actor, authority, resource, Operation::Verify)?;
        validate_policy(image_kinds, minimum_abi, maximum_abi)?;
        let signer_id = agent_image_signer_id(public_key);
        if self.find_agent_image_signer_index(signer_id).is_some() {
            return Err(KernelError::AgentImageSignerAlreadyExists);
        }
        if self.agent_image_signer_len >= AGENT_IMAGES {
            return Err(KernelError::AgentImageSignerStoreFull);
        }
        self.ensure_event_slots(1)?;
        let generation = self
            .agent_image_signer_policy_generation
            .checked_add(1)
            .ok_or(KernelError::AgentImageSignerGenerationExhausted)?;
        let record = AgentImageSignerRecord {
            signer_id,
            resource,
            public_key,
            image_kinds,
            minimum_abi,
            maximum_abi,
            status: AgentImageSignerStatus::Active,
            generation,
        };

        self.agent_image_signers[self.agent_image_signer_len] = record;
        self.agent_image_signer_len += 1;
        self.agent_image_signer_policy_generation = generation;
        self.record_agent_image_signer_event(
            actor,
            authority,
            resource,
            EventKind::AgentImageSignerTrusted,
            Operation::Verify,
            record,
            None,
        )?;
        Ok(record)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn rotate_agent_image_signer(
        &mut self,
        actor: AgentId,
        authority: CapabilityId,
        resource: ResourceId,
        expected_generation: u64,
        previous_signer_id: AgentImageSignerId,
        replacement_public_key: [u8; 32],
        replacement_image_kinds: AgentImageKindScope,
        replacement_minimum_abi: u16,
        replacement_maximum_abi: u16,
    ) -> Result<AgentImageSignerRotation, KernelError> {
        self.ensure_agent_active(actor)?;
        self.ensure_authorized(actor, authority, resource, Operation::Verify)?;
        self.ensure_authorized(actor, authority, resource, Operation::Rollback)?;
        if expected_generation == 0
            || expected_generation != self.agent_image_signer_policy_generation
        {
            return Err(KernelError::AgentImageSignerGenerationStale);
        }
        validate_policy(
            replacement_image_kinds,
            replacement_minimum_abi,
            replacement_maximum_abi,
        )?;
        let previous_index = self
            .find_agent_image_signer_index(previous_signer_id)
            .ok_or(KernelError::AgentImageSignerNotFound)?;
        let previous = self.agent_image_signers[previous_index];
        if previous.resource != resource {
            return Err(KernelError::AgentImageSignerResourceMismatch);
        }
        if previous.status != AgentImageSignerStatus::Active {
            return Err(KernelError::AgentImageSignerStatusMismatch);
        }
        let replacement_signer_id = agent_image_signer_id(replacement_public_key);
        if self
            .find_agent_image_signer_index(replacement_signer_id)
            .is_some()
        {
            return Err(KernelError::AgentImageSignerAlreadyExists);
        }
        if self.agent_image_signer_len >= AGENT_IMAGES {
            return Err(KernelError::AgentImageSignerStoreFull);
        }
        self.ensure_event_slots(2)?;
        let generation = self
            .agent_image_signer_policy_generation
            .checked_add(1)
            .ok_or(KernelError::AgentImageSignerGenerationExhausted)?;
        let replacement = AgentImageSignerRecord {
            signer_id: replacement_signer_id,
            resource,
            public_key: replacement_public_key,
            image_kinds: replacement_image_kinds,
            minimum_abi: replacement_minimum_abi,
            maximum_abi: replacement_maximum_abi,
            status: AgentImageSignerStatus::Active,
            generation,
        };
        let revoked = AgentImageSignerRecord {
            status: AgentImageSignerStatus::Revoked,
            generation,
            ..previous
        };

        self.agent_image_signers[self.agent_image_signer_len] = replacement;
        self.agent_image_signer_len += 1;
        self.agent_image_signers[previous_index] = revoked;
        self.agent_image_signer_policy_generation = generation;
        self.record_agent_image_signer_event(
            actor,
            authority,
            resource,
            EventKind::AgentImageSignerTrusted,
            Operation::Verify,
            replacement,
            Some(previous_signer_id),
        )?;
        self.record_agent_image_signer_event(
            actor,
            authority,
            resource,
            EventKind::AgentImageSignerRevoked,
            Operation::Rollback,
            revoked,
            Some(replacement_signer_id),
        )?;
        Ok(AgentImageSignerRotation::new(
            revoked,
            replacement,
            generation,
        ))
    }

    pub fn agent_image_signers(&self) -> &[AgentImageSignerRecord] {
        &self.agent_image_signers[..self.agent_image_signer_len]
    }

    pub fn agent_image_signer(
        &self,
        signer_id: AgentImageSignerId,
    ) -> Result<AgentImageSignerRecord, KernelError> {
        self.find_agent_image_signer_index(signer_id)
            .map(|index| self.agent_image_signers[index])
            .ok_or(KernelError::AgentImageSignerNotFound)
    }

    pub const fn agent_image_signer_policy_generation(&self) -> u64 {
        self.agent_image_signer_policy_generation
    }

    fn find_agent_image_signer_index(&self, signer_id: AgentImageSignerId) -> Option<usize> {
        self.agent_image_signers()
            .iter()
            .position(|record| record.signer_id == signer_id)
    }
}

fn validate_policy(
    image_kinds: AgentImageKindScope,
    minimum_abi: u16,
    maximum_abi: u16,
) -> Result<(), KernelError> {
    if image_kinds.is_empty() || minimum_abi == 0 || minimum_abi > maximum_abi {
        Err(KernelError::AgentImageSignerPolicyInvalid)
    } else {
        Ok(())
    }
}
