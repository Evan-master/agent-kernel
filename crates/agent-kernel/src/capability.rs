//! Capability syscall facade.
//!
//! This module belongs to `agent-kernel`. It exposes root capability grants and
//! least-authority capability derivation while keeping validation and event
//! recording inside `agent-kernel-core`.

use agent_kernel_core::{AgentId, Capability, CapabilityId, KernelError, OperationSet, ResourceId};

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
    >
{
    pub fn sys_grant(
        &mut self,
        agent: AgentId,
        resource: ResourceId,
        operations: OperationSet,
    ) -> Result<CapabilityId, KernelError> {
        self.core.grant_capability(agent, resource, operations)
    }

    pub fn sys_derive_capability(
        &mut self,
        actor: AgentId,
        source_capability: CapabilityId,
        target_agent: AgentId,
        operations: OperationSet,
    ) -> Result<CapabilityId, KernelError> {
        self.core
            .derive_capability(actor, source_capability, target_agent, operations)
    }

    pub fn capability(&self, capability: CapabilityId) -> Result<Capability, KernelError> {
        self.core.capability(capability)
    }
}
