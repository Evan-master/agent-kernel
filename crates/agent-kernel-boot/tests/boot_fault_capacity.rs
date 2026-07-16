use agent_kernel_boot::{BootConfig, BootedKernel};
use agent_kernel_core::{
    AgentEntryKind, AgentExecutionState, AgentId, AgentImageDigest, AgentImageKind, EventKind,
    FaultKind, IntentKind, TaskStatus, VerificationRequirement,
};

type FaultBoot = BootedKernel<2, 1, 2, 24, 1, 1, 0, 1, 1, 1, 0, 0, 0, 0, 0, 0, 1>;

#[test]
fn booted_kernel_exposes_fault_and_owner_recovery_to_the_public_facade() {
    let mut booted = FaultBoot::boot(BootConfig::default()).unwrap();
    let report = *booted.report();
    let worker = AgentId::new(3);
    let kernel = booted.kernel_mut();

    kernel.sys_register_agent(worker).unwrap();
    let intent = kernel
        .sys_declare_intent(
            report.bootstrap_agent,
            report.bootstrap_capability,
            report.bootstrap_resource,
            IntentKind::Act,
            VerificationRequirement::Required,
        )
        .unwrap();
    let task = kernel
        .sys_create_task(report.bootstrap_agent, report.bootstrap_capability, intent)
        .unwrap();
    kernel
        .sys_delegate_task(
            report.bootstrap_agent,
            report.bootstrap_capability,
            task,
            worker,
        )
        .unwrap();
    let capability = kernel.tasks()[0].delegated_capability.unwrap();
    let image = kernel
        .sys_register_agent_image(
            report.bootstrap_agent,
            report.bootstrap_capability,
            report.bootstrap_resource,
            AgentImageKind::Worker,
            AgentImageDigest::new([0x46; 32]),
            1,
            1,
        )
        .unwrap();
    kernel
        .sys_verify_agent_image(report.bootstrap_agent, report.bootstrap_capability, image)
        .unwrap();
    kernel
        .sys_launch_task_agent(worker, capability, task, image, AgentEntryKind::Worker)
        .unwrap();
    kernel.sys_accept_task(worker, task).unwrap();
    kernel.sys_enqueue_task(worker, task).unwrap();
    assert_eq!(kernel.sys_dispatch_next_with_quantum(worker, 1), Ok(task));

    let fault = kernel
        .sys_fault_task(worker, task, FaultKind::ExecutionTrap, 6)
        .unwrap();

    assert_eq!(kernel.faults().len(), 1);
    assert_eq!(kernel.faults()[0].id, fault);
    assert_eq!(kernel.faults()[0].kind, FaultKind::ExecutionTrap);
    assert_eq!(kernel.faults()[0].detail, 6);
    assert_eq!(kernel.tasks()[0].status, TaskStatus::Faulted);
    assert_eq!(kernel.tasks()[0].last_fault, Some(fault));
    let context = kernel
        .execution_contexts()
        .iter()
        .find(|context| context.agent == worker)
        .unwrap();
    assert_eq!(context.state, AgentExecutionState::Faulted);
    assert_eq!(context.task, Some(task));
    let event = kernel.events().last().unwrap();
    assert_eq!(event.kind, EventKind::TaskFaulted);
    assert_eq!(event.fault, Some(fault));
    assert_eq!(event.fault_kind, Some(FaultKind::ExecutionTrap));
    assert_eq!(event.fault_detail, Some(6));

    let recovered = kernel
        .sys_recover_faulted_task(report.bootstrap_agent, report.bootstrap_capability, task)
        .unwrap();
    assert_eq!(recovered.kind, EventKind::TaskFaultRecovered);
    assert_eq!(recovered.fault, Some(fault));
    assert_eq!(kernel.tasks()[0].status, TaskStatus::Accepted);
    let context = kernel
        .execution_contexts()
        .iter()
        .find(|context| context.agent == worker)
        .unwrap();
    assert_eq!(context.state, AgentExecutionState::Idle);
    assert_eq!(context.task, None);
}
