//! Driver Endpoint registration syscall facade.
//!
//! This `agent-kernel` module exposes endpoint installation and read-only
//! inspection while delegating descriptor validation, authorization, storage,
//! and audit events to `agent-kernel-core`. It performs no architecture I/O.

use agent_kernel_core::{
    AgentId, CapabilityId, DriverEndpointDescriptor, DriverEndpointRecord, Event, KernelError,
    ResourceId,
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
    pub fn sys_register_driver_endpoint(
        &mut self,
        installer: AgentId,
        capability: CapabilityId,
        resource: ResourceId,
        descriptor: DriverEndpointDescriptor,
    ) -> Result<Event, KernelError> {
        self.core
            .register_driver_endpoint(installer, capability, resource, descriptor)
    }

    pub fn driver_endpoints(&self) -> &[DriverEndpointRecord] {
        self.core.driver_endpoints()
    }

    pub fn driver_endpoint(
        &self,
        resource: ResourceId,
    ) -> Result<DriverEndpointRecord, KernelError> {
        self.core.driver_endpoint(resource)
    }
}
