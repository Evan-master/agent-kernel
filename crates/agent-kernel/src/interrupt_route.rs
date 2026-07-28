//! Interrupt Route authority syscall facade.
//!
//! This no_std facade exposes Core route reservation, activation, revocation,
//! and inspection. Architecture owners pair transitions with PCI source
//! programming and interrupt-controller state.

use agent_kernel_core::{
    AgentId, CapabilityId, Event, InterruptMode, InterruptRouteRecord, InterruptTarget,
    KernelError, OperationSet, ResourceCreateOutcome, ResourceId,
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
    pub fn sys_create_interrupt_route(
        &mut self,
        agent: AgentId,
        device_capability: CapabilityId,
        device: ResourceId,
        mode: InterruptMode,
        target: InterruptTarget,
        operations: OperationSet,
    ) -> Result<ResourceCreateOutcome, KernelError> {
        self.core
            .create_interrupt_route(agent, device_capability, device, mode, target, operations)
    }

    pub fn sys_activate_interrupt_route(
        &mut self,
        agent: AgentId,
        capability: CapabilityId,
        route: ResourceId,
    ) -> Result<Event, KernelError> {
        self.core.activate_interrupt_route(agent, capability, route)
    }

    pub fn sys_begin_interrupt_route_revoke(
        &mut self,
        agent: AgentId,
        capability: CapabilityId,
        route: ResourceId,
    ) -> Result<Event, KernelError> {
        self.core
            .begin_interrupt_route_revoke(agent, capability, route)
    }

    pub fn sys_complete_interrupt_route_revoke(
        &mut self,
        agent: AgentId,
        capability: CapabilityId,
        route: ResourceId,
    ) -> Result<Event, KernelError> {
        self.core
            .complete_interrupt_route_revoke(agent, capability, route)
    }

    pub fn interrupt_routes(&self) -> &[InterruptRouteRecord] {
        self.core.interrupt_routes()
    }

    pub fn interrupt_route(&self, route: ResourceId) -> Result<InterruptRouteRecord, KernelError> {
        self.core.interrupt_route(route)
    }
}
