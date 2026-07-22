//! Syscall facade for runtime signed-image Trust Policy mutations.
//!
//! This no_std facade module forwards capability-authorized initial trust and
//! atomic rotation to `agent-kernel-core`. It exposes read-only policy state
//! and cannot mutate records outside the deterministic core transition.

use agent_kernel_core::{
    AgentId, AgentImageKindScope, AgentImageSignerId, AgentImageSignerRecord,
    AgentImageSignerRotation, CapabilityId, KernelError, ResourceId,
};

use crate::AgentKernel;

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
    AgentKernel<
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
    #[allow(clippy::too_many_arguments)]
    pub fn sys_trust_agent_image_signer(
        &mut self,
        actor: AgentId,
        authority: CapabilityId,
        resource: ResourceId,
        public_key: [u8; 32],
        image_kinds: AgentImageKindScope,
        minimum_abi: u16,
        maximum_abi: u16,
    ) -> Result<AgentImageSignerRecord, KernelError> {
        self.core.trust_agent_image_signer(
            actor,
            authority,
            resource,
            public_key,
            image_kinds,
            minimum_abi,
            maximum_abi,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn sys_rotate_agent_image_signer(
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
        self.core.rotate_agent_image_signer(
            actor,
            authority,
            resource,
            expected_generation,
            previous_signer_id,
            replacement_public_key,
            replacement_image_kinds,
            replacement_minimum_abi,
            replacement_maximum_abi,
        )
    }

    pub fn agent_image_signers(&self) -> &[AgentImageSignerRecord] {
        self.core.agent_image_signers()
    }

    pub fn agent_image_signer(
        &self,
        signer_id: AgentImageSignerId,
    ) -> Result<AgentImageSignerRecord, KernelError> {
        self.core.agent_image_signer(signer_id)
    }

    pub const fn agent_image_signer_policy_generation(&self) -> u64 {
        self.core.agent_image_signer_policy_generation()
    }
}
