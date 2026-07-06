//! Resource lifecycle syscall facade.
//!
//! This module belongs to `agent-kernel`. It exposes resource registration,
//! retirement, and inspection while keeping lifecycle mutation inside
//! `agent-kernel-core`.

use agent_kernel_core::{
    AgentId, CapabilityId, Event, KernelError, Resource, ResourceId, ResourceKind,
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
    >
{
    pub fn sys_register_resource(
        &mut self,
        kind: ResourceKind,
        parent: Option<ResourceId>,
    ) -> Result<ResourceId, KernelError> {
        self.core.register_resource(kind, parent)
    }

    pub fn sys_retire_resource(
        &mut self,
        agent: AgentId,
        capability: CapabilityId,
        resource: ResourceId,
    ) -> Result<Event, KernelError> {
        self.core.retire_resource(agent, capability, resource)
    }

    pub fn resources(&self) -> &[Resource] {
        self.core.resources()
    }
}
