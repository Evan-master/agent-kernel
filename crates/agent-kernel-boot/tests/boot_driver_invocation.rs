use agent_kernel_boot::{BootConfig, BootedKernel};
use agent_kernel_core::{
    AgentEntryKind, AgentExecutionState, AgentId, AgentImageDigest, AgentImageKind,
    DeviceEventKind, DeviceEventPayload, DeviceEventStatus, DriverCommandKind,
    DriverCommandPayload, DriverCommandResult, DriverCommandStatus, DriverEndpointDescriptor,
    DriverInvocationStatus, Operation, OperationSet,
};

type DriverBoot = BootedKernel<2, 1, 2, 32, 1, 1, 0, 0, 0, 0, 1, 1, 1, 1>;

#[test]
fn booted_kernel_completes_interrupt_causal_driver_invocation() {
    let mut booted = DriverBoot::boot(BootConfig::default()).unwrap();
    let report = *booted.report();
    let driver = AgentId::new(2);
    let kernel = booted.kernel_mut();

    kernel
        .sys_register_driver_endpoint(
            report.bootstrap_agent,
            report.bootstrap_capability,
            report.bootstrap_resource,
            DriverEndpointDescriptor::port(0x3f8, 8),
        )
        .unwrap();
    kernel.sys_register_agent(driver).unwrap();
    let driver_capability = kernel
        .sys_derive_capability(
            report.bootstrap_agent,
            report.bootstrap_capability,
            driver,
            OperationSet::empty()
                .with(Operation::Observe)
                .with(Operation::Act),
        )
        .unwrap();
    let image = kernel
        .sys_register_agent_image(
            report.bootstrap_agent,
            report.bootstrap_capability,
            report.bootstrap_resource,
            AgentImageKind::Driver,
            AgentImageDigest::new([0x44; 32]),
            1,
            1,
        )
        .unwrap();
    kernel
        .sys_verify_agent_image(report.bootstrap_agent, report.bootstrap_capability, image)
        .unwrap();
    kernel
        .sys_launch_agent(
            driver,
            driver_capability,
            report.bootstrap_resource,
            image,
            AgentEntryKind::Driver,
            None,
        )
        .unwrap();
    kernel
        .sys_bind_driver(
            report.bootstrap_agent,
            report.bootstrap_capability,
            report.bootstrap_resource,
            driver,
        )
        .unwrap();

    let event = kernel
        .sys_raise_device_event(
            report.bootstrap_agent,
            report.bootstrap_capability,
            report.bootstrap_resource,
            DeviceEventKind::Interrupt,
            DeviceEventPayload {
                code: 0xc2,
                value: 0x20,
            },
        )
        .unwrap();
    let invocation = kernel
        .sys_deliver_device_event(driver, driver_capability, event)
        .unwrap();
    assert_eq!(
        kernel.sys_dispatch_next_driver_invocation(driver, 2),
        Ok(invocation)
    );
    kernel
        .sys_tick_driver_invocation(driver, invocation)
        .unwrap();
    kernel
        .sys_acknowledge_device_event(driver, driver_capability, event)
        .unwrap();

    let write = kernel
        .sys_submit_driver_command(
            driver,
            driver_capability,
            report.bootstrap_resource,
            Some(event),
            DriverCommandKind::Write,
            DriverCommandPayload {
                opcode: 0,
                value: u64::from(b'O'),
            },
        )
        .unwrap();
    let write_request = kernel
        .sys_dispatch_driver_command(driver, driver_capability, write)
        .unwrap();
    assert_eq!(write_request.cause, Some(event));
    assert_eq!(write_request.invocation, Some(invocation));
    kernel
        .sys_complete_driver_command(
            driver,
            driver_capability,
            write,
            DriverCommandResult {
                code: 0,
                value: u64::from(b'O'),
            },
        )
        .unwrap();
    kernel
        .sys_complete_driver_invocation(driver, driver_capability, invocation)
        .unwrap();

    assert_eq!(
        kernel.device_events()[0].status,
        DeviceEventStatus::Acknowledged
    );
    assert_eq!(kernel.driver_commands().len(), 1);
    assert_eq!(kernel.driver_commands()[0].cause, Some(event));
    assert_eq!(
        kernel.driver_commands()[0].status,
        DriverCommandStatus::Completed
    );
    assert_eq!(
        kernel.driver_invocations()[0].status,
        DriverInvocationStatus::Completed
    );
    assert_eq!(kernel.driver_invocations()[0].run_ticks, 1);
    assert_eq!(
        kernel
            .execution_contexts()
            .iter()
            .find(|context| context.agent == driver)
            .unwrap()
            .state,
        AgentExecutionState::Idle
    );
}
