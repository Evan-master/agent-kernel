use agent_kernel_boot::{BootConfig, BootPhase, BootedKernel};
use agent_kernel_core::{
    ActionId, AgentEntryKind, AgentId, AgentImageDigest, AgentImageId, AgentImageKind,
    DriverCommandKind, DriverCommandPayload, DriverCommandResult, DriverCommandStatus,
    DriverEndpointDescriptor, EventKind, Operation, OperationSet,
};

#[test]
fn boot_records_phase_sequence() {
    let booted = BootedKernel::<2, 8, 8, 16, 4, 4, 4, 0, 4, 4>::boot(BootConfig::default())
        .expect("boot flow should fit fixed stores");

    assert_eq!(
        booted.report().phases,
        [
            BootPhase::EnteredKernel,
            BootPhase::KernelInitialized,
            BootPhase::SupervisorHandoffReady,
        ]
    );
    assert_eq!(booted.report().bootstrap_image, AgentImageId::new(1));
}

#[test]
fn boot_records_observe_action_and_verify_events() {
    let config = BootConfig::default().with_boot_action(ActionId::new(99));
    let booted = BootedKernel::<2, 8, 8, 16, 4, 4, 4, 0, 4, 4>::boot(config)
        .expect("boot flow should fit fixed stores");

    let events = booted.kernel().events();
    assert_eq!(events.len(), 8);
    assert_eq!(events[0].kind, EventKind::AgentRegistered);
    assert_eq!(events[1].kind, EventKind::CapabilityGranted);
    assert_eq!(events[2].kind, EventKind::AgentImageRegistered);
    assert_eq!(events[3].kind, EventKind::AgentImageVerified);
    assert_eq!(events[4].kind, EventKind::AgentLaunched);
    assert_eq!(events[5].kind, EventKind::Observation);
    assert_eq!(events[6].kind, EventKind::ActionExecuted);
    assert_eq!(events[7].kind, EventKind::VerificationRequested);
    assert_eq!(events[6].action, Some(ActionId::new(99)));
    assert_eq!(events[7].action, Some(ActionId::new(99)));
}

#[test]
fn trusted_boot_handoff_can_register_architecture_endpoint() {
    let mut booted = BootedKernel::<2, 8, 8, 16, 4, 4, 4, 0, 4, 4>::boot(BootConfig::default())
        .expect("boot flow should fit fixed stores");
    let report = *booted.report();
    let descriptor = DriverEndpointDescriptor::port(0x3f8, 8);

    let event = booted
        .kernel_mut()
        .sys_register_driver_endpoint(
            report.bootstrap_agent,
            report.bootstrap_capability,
            report.bootstrap_resource,
            descriptor,
        )
        .expect("bootstrap authority should install architecture endpoint");

    assert_eq!(event.kind, EventKind::DriverEndpointRegistered);
    assert_eq!(
        booted
            .kernel()
            .driver_endpoint(report.bootstrap_resource)
            .unwrap()
            .descriptor,
        descriptor
    );
}

#[test]
fn booted_kernel_with_driver_capacity_completes_cause_free_command() {
    type DriverBoot = BootedKernel<2, 1, 2, 20, 1, 1, 0, 0, 0, 0, 1, 0, 1, 0>;

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
            OperationSet::only(Operation::Act),
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
    let command = kernel
        .sys_submit_driver_command(
            driver,
            driver_capability,
            report.bootstrap_resource,
            None,
            DriverCommandKind::Write,
            DriverCommandPayload {
                opcode: 0,
                value: u64::from(b'O'),
            },
        )
        .unwrap();
    let request = kernel
        .sys_dispatch_driver_command(driver, driver_capability, command)
        .unwrap();
    assert_eq!(request.command, command);
    kernel
        .sys_complete_driver_command(
            driver,
            driver_capability,
            command,
            DriverCommandResult {
                code: 0,
                value: u64::from(b'O'),
            },
        )
        .unwrap();

    assert_eq!(
        kernel.driver_commands()[0].status,
        DriverCommandStatus::Completed
    );
}
