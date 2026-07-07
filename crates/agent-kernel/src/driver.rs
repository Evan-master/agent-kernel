//! Driver binding syscall facade.
//!
//! This module belongs to `agent-kernel`. It exposes driver binding and device
//! event lifecycle syscalls while keeping all authority and state mutation in
//! `agent-kernel-core`.

use agent_kernel_core::{
    AgentId, CapabilityId, DeviceEventId, DeviceEventKind, DeviceEventPayload, DeviceEventRecord,
    DriverBindingId, DriverBindingRecord, Event, KernelError, ResourceId,
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
    >
{
    pub fn sys_bind_driver(
        &mut self,
        installer: AgentId,
        capability: CapabilityId,
        resource: ResourceId,
        driver: AgentId,
    ) -> Result<DriverBindingId, KernelError> {
        self.core
            .bind_driver(installer, capability, resource, driver)
    }

    pub fn sys_raise_device_event(
        &mut self,
        agent: AgentId,
        capability: CapabilityId,
        resource: ResourceId,
        kind: DeviceEventKind,
        payload: DeviceEventPayload,
    ) -> Result<DeviceEventId, KernelError> {
        self.core
            .raise_device_event(agent, capability, resource, kind, payload)
    }

    pub fn sys_deliver_device_event(
        &mut self,
        driver: AgentId,
        capability: CapabilityId,
        event: DeviceEventId,
    ) -> Result<Event, KernelError> {
        self.core.deliver_device_event(driver, capability, event)
    }

    pub fn sys_acknowledge_device_event(
        &mut self,
        driver: AgentId,
        capability: CapabilityId,
        event: DeviceEventId,
    ) -> Result<Event, KernelError> {
        self.core
            .acknowledge_device_event(driver, capability, event)
    }

    pub fn driver_bindings(&self) -> &[DriverBindingRecord] {
        self.core.driver_bindings()
    }

    pub fn device_events(&self) -> &[DeviceEventRecord] {
        self.core.device_events()
    }
}
