//! Resource-oriented supervisor flow after the task lifecycle completes.
//!
//! This supervisor-layer module keeps host-side demonstration code out of the
//! kernel crates. It drives only syscall-style facade methods, owns no kernel
//! internals directly, and exists to keep `main.rs` focused on the primary task
//! lifecycle flow.

use agent_kernel::AgentKernel;
use agent_kernel_core::{
    AgentEntryKind, AgentId, AgentImageDigest, AgentImageKind, CapabilityId, DeviceEventKind,
    DeviceEventPayload, DriverCommandKind, DriverCommandPayload, DriverEndpointDescriptor,
    MemoryValue, NamespaceKey, NamespaceObject, Operation, OperationSet, ResourceId, ResourceKind,
    TaskId,
};
use agent_kernel_hal::{DriverBackend, DriverCommandOutcome};

use crate::virtual_device::VirtualRegisterDevice;

pub type SupervisorKernel =
    AgentKernel<8, 8, 8, 96, 8, 8, 8, 8, 8, 8, 8, 8, 8, 1, 1, 1, 1, 8, 2, 2, 2, 2>;

pub struct ResourceFlowContext {
    pub agent: AgentId,
    pub target_agent: AgentId,
    pub owner_capability: CapabilityId,
    pub workspace: ResourceId,
    pub task: TaskId,
}

pub fn drive_resource_flow(kernel: &mut SupervisorKernel, context: ResourceFlowContext) {
    let memory = kernel
        .sys_register_resource(ResourceKind::Memory, None)
        .expect("memory resource should fit in simulator kernel");
    let memory_capability = kernel
        .sys_grant(
            context.agent,
            memory,
            OperationSet::empty()
                .with(Operation::Observe)
                .with(Operation::Act),
        )
        .expect("memory capability should fit in simulator kernel");
    let memory_cell = kernel
        .sys_create_memory_cell(
            context.agent,
            memory_capability,
            memory,
            MemoryValue::new([1, 2, 3, 4]),
        )
        .expect("memory cell should fit in simulator kernel");
    let recalled = kernel
        .sys_recall_memory_cell(context.agent, memory_capability, memory_cell)
        .expect("agent should recall memory cell");
    assert_eq!(recalled, MemoryValue::new([1, 2, 3, 4]));
    kernel
        .sys_remember_memory_cell(
            context.agent,
            memory_capability,
            memory_cell,
            MemoryValue::new([4, 3, 2, 1]),
        )
        .expect("agent should remember new memory cell value");

    let namespace_key = NamespaceKey::new(1);
    let namespace_entry = kernel
        .sys_bind_namespace_entry(
            context.agent,
            context.owner_capability,
            context.workspace,
            namespace_key,
            NamespaceObject::MemoryCell(memory_cell),
        )
        .expect("agent should bind memory cell in workspace namespace");
    let resolved = kernel
        .sys_resolve_namespace_entry(
            context.agent,
            context.owner_capability,
            context.workspace,
            namespace_key,
        )
        .expect("agent should resolve workspace namespace entry");
    assert_eq!(resolved, NamespaceObject::MemoryCell(memory_cell));
    kernel
        .sys_rebind_namespace_entry(
            context.agent,
            context.owner_capability,
            namespace_entry,
            NamespaceObject::Task(context.task),
        )
        .expect("agent should rebind namespace entry to task");

    let service = kernel
        .sys_create_resource(
            context.agent,
            ResourceKind::Service,
            Some((context.workspace, context.owner_capability)),
            OperationSet::only(Operation::Rollback),
        )
        .expect("owned service resource should fit in simulator kernel");
    kernel
        .sys_retire_resource(context.agent, service.capability, service.resource)
        .expect("agent should retire service resource");
    let target_observe_capability = kernel
        .sys_derive_capability(
            context.agent,
            context.owner_capability,
            context.target_agent,
            OperationSet::only(Operation::Observe),
        )
        .expect("owner should derive observe authority to target agent");
    kernel
        .sys_observe(
            context.target_agent,
            target_observe_capability,
            context.workspace,
        )
        .expect("target agent should observe through derived capability");
}

pub fn drive_driver_flow(kernel: &mut SupervisorKernel, context: ResourceFlowContext) {
    let driver_agent = AgentId::new(4);
    kernel
        .sys_register_agent(driver_agent)
        .expect("driver agent should fit in simulator kernel");
    let device = kernel
        .sys_create_resource(
            context.agent,
            ResourceKind::Device,
            Some((context.workspace, context.owner_capability)),
            OperationSet::empty()
                .with(Operation::Observe)
                .with(Operation::Act)
                .with(Operation::Delegate)
                .with(Operation::Verify),
        )
        .expect("owned device resource should fit in simulator kernel");
    kernel
        .sys_register_driver_endpoint(
            context.agent,
            device.capability,
            device.resource,
            DriverEndpointDescriptor::virtual_channel(1),
        )
        .expect("owner should register virtual device endpoint");
    let endpoint = kernel
        .driver_endpoint(device.resource)
        .expect("registered device endpoint should resolve");
    let mut backend =
        VirtualRegisterDevice::new(endpoint).expect("virtual endpoint should configure backend");
    assert_eq!(backend.channel(), 1);
    let driver_capability = kernel
        .sys_derive_capability(
            context.agent,
            device.capability,
            driver_agent,
            OperationSet::empty()
                .with(Operation::Observe)
                .with(Operation::Act),
        )
        .expect("owner should derive device driver authority");
    let driver_image = kernel
        .sys_register_agent_image(
            context.agent,
            device.capability,
            device.resource,
            AgentImageKind::Driver,
            AgentImageDigest::new([3; 32]),
            1,
            1,
        )
        .expect("driver image should register");
    kernel
        .sys_verify_agent_image(context.agent, device.capability, driver_image)
        .expect("driver image should verify");
    kernel
        .sys_launch_agent(
            driver_agent,
            driver_capability,
            device.resource,
            driver_image,
            AgentEntryKind::Driver,
            None,
        )
        .expect("driver agent should launch into device entry");
    kernel
        .sys_bind_driver(
            context.agent,
            device.capability,
            device.resource,
            driver_agent,
        )
        .expect("owner should bind target as device driver");
    let event = kernel
        .sys_raise_device_event(
            context.agent,
            device.capability,
            device.resource,
            DeviceEventKind::StateChanged,
            DeviceEventPayload { code: 1, value: 2 },
        )
        .expect("owner should raise simulated device event");
    let invocation = kernel
        .sys_deliver_device_event(driver_agent, driver_capability, event)
        .expect("driver should receive device event");
    kernel
        .sys_dispatch_next_driver_invocation(driver_agent, 2)
        .expect("driver invocation should dispatch");
    kernel
        .sys_tick_driver_invocation(driver_agent, invocation)
        .expect("driver invocation should advance one tick");
    kernel
        .sys_acknowledge_device_event(driver_agent, driver_capability, event)
        .expect("driver should acknowledge device event");
    let command = kernel
        .sys_submit_driver_command(
            driver_agent,
            driver_capability,
            device.resource,
            Some(event),
            DriverCommandKind::Write,
            DriverCommandPayload {
                opcode: 3,
                value: 11,
            },
        )
        .expect("driver should submit device command");
    let request = kernel
        .sys_dispatch_driver_command(driver_agent, driver_capability, command)
        .expect("kernel should dispatch authorized device command");
    match backend.execute(request) {
        DriverCommandOutcome::Completed(result) => kernel
            .sys_complete_driver_command(driver_agent, driver_capability, command, result)
            .expect("kernel should record completed backend outcome"),
        DriverCommandOutcome::Failed(result) => kernel
            .sys_fail_driver_command(driver_agent, driver_capability, command, result)
            .expect("kernel should record failed backend outcome"),
    };
    assert_eq!(backend.value(), 11);
    assert_eq!(backend.executions(), 1);
    kernel
        .sys_complete_driver_invocation(driver_agent, driver_capability, invocation)
        .expect("driver invocation should complete");
}
