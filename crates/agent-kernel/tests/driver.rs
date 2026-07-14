use agent_kernel::AgentKernel;
use agent_kernel_core::{
    AgentId, DeviceEventKind, DeviceEventPayload, DeviceEventStatus, DriverCommandKind,
    DriverCommandPayload, DriverCommandResult, DriverCommandStatus, EventKind, Operation,
    OperationSet, ResourceKind,
};

type TestKernel = AgentKernel<4, 4, 6, 12, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 2, 2, 2>;

#[test]
fn driver_syscalls_expose_event_and_command_lifecycles() {
    let mut kernel = TestKernel::new();
    let owner = AgentId::new(1);
    let driver = AgentId::new(2);
    kernel.sys_register_agent(owner).unwrap();
    kernel.sys_register_agent(driver).unwrap();
    let device = kernel
        .sys_register_resource(ResourceKind::Device, None)
        .unwrap();
    let owner_capability = kernel
        .sys_grant(
            owner,
            device,
            OperationSet::empty()
                .with(Operation::Delegate)
                .with(Operation::Act),
        )
        .unwrap();
    let driver_capability = kernel
        .sys_grant(
            driver,
            device,
            OperationSet::empty()
                .with(Operation::Observe)
                .with(Operation::Act),
        )
        .unwrap();
    let binding = kernel
        .sys_bind_driver(owner, owner_capability, device, driver)
        .unwrap();
    let event = kernel
        .sys_raise_device_event(
            owner,
            owner_capability,
            device,
            DeviceEventKind::StateChanged,
            DeviceEventPayload { code: 1, value: 2 },
        )
        .unwrap();
    kernel
        .sys_deliver_device_event(driver, driver_capability, event)
        .unwrap();
    kernel
        .sys_acknowledge_device_event(driver, driver_capability, event)
        .unwrap();
    let command = kernel
        .sys_submit_driver_command(
            driver,
            driver_capability,
            device,
            Some(event),
            DriverCommandKind::Write,
            DriverCommandPayload {
                opcode: 3,
                value: 11,
            },
        )
        .unwrap();
    kernel
        .sys_complete_driver_command(
            driver,
            driver_capability,
            command,
            DriverCommandResult { code: 0, value: 12 },
        )
        .unwrap();

    assert_eq!(kernel.driver_bindings()[0].id, binding);
    assert_eq!(
        kernel.device_events()[0].status,
        DeviceEventStatus::Acknowledged
    );
    assert_eq!(
        kernel.driver_commands()[0].status,
        DriverCommandStatus::Completed
    );
    assert_eq!(
        kernel.events().last().unwrap().kind,
        EventKind::DriverCommandCompleted
    );
}
