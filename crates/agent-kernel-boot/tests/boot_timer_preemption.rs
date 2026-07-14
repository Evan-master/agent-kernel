use agent_kernel_boot::{BootConfig, BootedKernel};
use agent_kernel_core::{
    AgentEntryKind, AgentExecutionState, AgentId, AgentImageDigest, AgentImageKind, EventKind,
    IntentKind, RunQueueEntry, TaskStatus, VerificationRequirement,
};

type TimerBoot = BootedKernel<2, 1, 2, 24, 1, 1, 0, 1, 1, 1>;

#[test]
fn booted_kernel_expires_running_worker_quantum_and_requeues_task() {
    let mut booted = TimerBoot::boot(BootConfig::default()).unwrap();
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
    let worker_capability = kernel.tasks()[0].delegated_capability.unwrap();
    let image = kernel
        .sys_register_agent_image(
            report.bootstrap_agent,
            report.bootstrap_capability,
            report.bootstrap_resource,
            AgentImageKind::Worker,
            AgentImageDigest::new([0x57; 32]),
            1,
            1,
        )
        .unwrap();
    kernel
        .sys_verify_agent_image(report.bootstrap_agent, report.bootstrap_capability, image)
        .unwrap();
    kernel
        .sys_launch_task_agent(
            worker,
            worker_capability,
            task,
            image,
            AgentEntryKind::Worker,
        )
        .unwrap();
    kernel.sys_accept_task(worker, task).unwrap();
    kernel.sys_enqueue_task(worker, task).unwrap();
    assert_eq!(kernel.sys_dispatch_next_with_quantum(worker, 1), Ok(task));

    let expiry = kernel.sys_tick_task(worker, task).unwrap();

    assert_eq!(expiry.kind, EventKind::TaskQuantumExpired);
    assert_eq!(expiry.task_ticks, Some(1));
    assert_eq!(expiry.task_quantum, Some(0));
    assert_eq!(kernel.tasks()[0].status, TaskStatus::Accepted);
    assert_eq!(kernel.tasks()[0].run_ticks, 1);
    assert_eq!(kernel.tasks()[0].quantum_remaining, 0);
    assert_eq!(
        kernel.run_queue(),
        &[RunQueueEntry {
            task,
            agent: worker,
        }]
    );
    let context = kernel
        .execution_contexts()
        .iter()
        .find(|context| context.agent == worker)
        .unwrap();
    assert_eq!(context.state, AgentExecutionState::Idle);
    assert_eq!(context.task, None);
    assert_eq!(kernel.events().len(), 21);
    assert_eq!(kernel.events()[19].kind, EventKind::TaskDispatched);
    assert_eq!(kernel.events()[20].kind, EventKind::TaskQuantumExpired);

    assert_eq!(kernel.sys_dispatch_next_with_quantum(worker, 1), Ok(task));
    let yielded = kernel.sys_yield_task(worker, task).unwrap();

    assert_eq!(yielded.kind, EventKind::TaskYielded);
    assert_eq!(kernel.tasks()[0].status, TaskStatus::Accepted);
    assert_eq!(kernel.tasks()[0].run_ticks, 1);
    assert_eq!(
        kernel.run_queue(),
        &[RunQueueEntry {
            task,
            agent: worker,
        }]
    );
    let context = kernel
        .execution_contexts()
        .iter()
        .find(|context| context.agent == worker)
        .unwrap();
    assert_eq!(context.state, AgentExecutionState::Idle);
    assert_eq!(context.task, None);
    assert_eq!(kernel.events().len(), 23);
    assert_eq!(kernel.events()[21].kind, EventKind::TaskDispatched);
    assert_eq!(kernel.events()[22].kind, EventKind::TaskYielded);
}
