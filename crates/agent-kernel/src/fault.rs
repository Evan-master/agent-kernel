//! Task fault syscall facade.
//!
//! This module belongs to `agent-kernel`. It exposes task fault and recovery
//! operations as boundary methods while keeping fixed-capacity fault mutation
//! inside `agent-kernel-core`.

use agent_kernel_core::{
    AgentId, CapabilityId, Event, FaultHandlerId, FaultHandlerRecord, FaultId, FaultKind,
    FaultPolicyAction, FaultPolicyId, FaultPolicyOutcome, FaultPolicyRecord, FaultRecord,
    KernelError, MessageId, ResourceId, TaskId,
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
    pub fn sys_fault_task(
        &mut self,
        agent: AgentId,
        task: TaskId,
        kind: FaultKind,
        detail: u64,
    ) -> Result<FaultId, KernelError> {
        self.core.fault_task(agent, task, kind, detail)
    }

    pub fn sys_recover_faulted_task(
        &mut self,
        agent: AgentId,
        capability: CapabilityId,
        task: TaskId,
    ) -> Result<Event, KernelError> {
        self.core.recover_faulted_task(agent, capability, task)
    }

    pub fn sys_install_fault_handler(
        &mut self,
        agent: AgentId,
        capability: CapabilityId,
        resource: ResourceId,
        kind: FaultKind,
        handler: AgentId,
    ) -> Result<FaultHandlerId, KernelError> {
        self.core
            .install_fault_handler(agent, capability, resource, kind, handler)
    }

    pub fn sys_route_fault_to_handler(
        &mut self,
        agent: AgentId,
        capability: CapabilityId,
        fault: FaultId,
    ) -> Result<MessageId, KernelError> {
        self.core.route_fault_to_handler(agent, capability, fault)
    }

    pub fn sys_install_fault_policy(
        &mut self,
        agent: AgentId,
        capability: CapabilityId,
        resource: ResourceId,
        kind: FaultKind,
        action: FaultPolicyAction,
    ) -> Result<FaultPolicyId, KernelError> {
        self.core
            .install_fault_policy(agent, capability, resource, kind, action)
    }

    pub fn sys_apply_fault_policy(
        &mut self,
        agent: AgentId,
        capability: CapabilityId,
        fault: FaultId,
    ) -> Result<FaultPolicyOutcome, KernelError> {
        self.core.apply_fault_policy(agent, capability, fault)
    }

    pub fn faults(&self) -> &[FaultRecord] {
        self.core.faults()
    }

    pub fn fault_handlers(&self) -> &[FaultHandlerRecord] {
        self.core.fault_handlers()
    }

    pub fn fault_policies(&self) -> &[FaultPolicyRecord] {
        self.core.fault_policies()
    }
}
