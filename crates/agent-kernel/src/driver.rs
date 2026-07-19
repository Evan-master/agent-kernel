//! Driver binding syscall facade.
//!
//! This module belongs to `agent-kernel`. It exposes driver binding, device
//! event, and driver command lifecycle syscalls while keeping all authority and
//! state mutation in `agent-kernel-core`.

use agent_kernel_core::{
    AgentId, CapabilityId, DeviceEventId, DeviceEventKind, DeviceEventPayload, DeviceEventRecord,
    DriverBindingId, DriverBindingRecord, DriverCommandId, DriverCommandKind, DriverCommandPayload,
    DriverCommandRecord, DriverCommandRequest, DriverCommandResult, DriverInvocationId, Event,
    KernelError, ResourceId,
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
    ) -> Result<DriverInvocationId, KernelError> {
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

    pub fn sys_submit_driver_command(
        &mut self,
        driver: AgentId,
        capability: CapabilityId,
        resource: ResourceId,
        cause: Option<DeviceEventId>,
        kind: DriverCommandKind,
        payload: DriverCommandPayload,
    ) -> Result<DriverCommandId, KernelError> {
        self.core
            .submit_driver_command(driver, capability, resource, cause, kind, payload)
    }

    pub fn sys_dispatch_driver_command(
        &mut self,
        driver: AgentId,
        capability: CapabilityId,
        command: DriverCommandId,
    ) -> Result<DriverCommandRequest, KernelError> {
        self.core
            .dispatch_driver_command(driver, capability, command)
    }

    pub fn sys_complete_driver_command(
        &mut self,
        driver: AgentId,
        capability: CapabilityId,
        command: DriverCommandId,
        result: DriverCommandResult,
    ) -> Result<Event, KernelError> {
        self.core
            .complete_driver_command(driver, capability, command, result)
    }

    pub fn sys_fail_driver_command(
        &mut self,
        driver: AgentId,
        capability: CapabilityId,
        command: DriverCommandId,
        result: DriverCommandResult,
    ) -> Result<Event, KernelError> {
        self.core
            .fail_driver_command(driver, capability, command, result)
    }

    pub fn driver_bindings(&self) -> &[DriverBindingRecord] {
        self.core.driver_bindings()
    }

    pub fn device_events(&self) -> &[DeviceEventRecord] {
        self.core.device_events()
    }

    pub fn driver_commands(&self) -> &[DriverCommandRecord] {
        self.core.driver_commands()
    }
}
