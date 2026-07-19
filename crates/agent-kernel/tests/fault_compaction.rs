use agent_kernel::AgentKernel;
use agent_kernel_core::{
    AgentEntryKind, AgentId, AgentImageDigest, AgentImageKind, FaultId, FaultKind, IntentKind,
    Operation, OperationSet, ResourceKind, VerificationRequirement,
};

type TestKernel = AgentKernel<1, 1, 3, 64, 0, 0, 0, 1, 1, 1, 0, 0, 0, 1, 0, 0, 0, 1>;

#[test]
fn facade_compacts_recovered_fault_and_reuses_slot_with_fresh_identity() {
    let mut kernel = TestKernel::new();
    let supervisor = AgentId::new(1);
    kernel.sys_register_agent(supervisor).unwrap();
    let resource = kernel
        .sys_register_resource(ResourceKind::Workspace, None)
        .unwrap();
    let authority = kernel
        .sys_grant(
            supervisor,
            resource,
            OperationSet::only(Operation::Act)
                .with(Operation::Verify)
                .with(Operation::Rollback)
                .with(Operation::Delegate),
        )
        .unwrap();
    let intent = kernel
        .sys_declare_intent(
            supervisor,
            authority,
            resource,
            IntentKind::Act,
            VerificationRequirement::Required,
        )
        .unwrap();
    let task = kernel
        .sys_create_task(supervisor, authority, intent)
        .unwrap();
    kernel
        .sys_delegate_task(supervisor, authority, task, supervisor)
        .unwrap();
    let task_authority = kernel.tasks()[0].delegated_capability.unwrap();
    let image = kernel
        .sys_register_agent_image(
            supervisor,
            authority,
            resource,
            AgentImageKind::Supervisor,
            AgentImageDigest::new([0x39; 32]),
            1,
            1,
        )
        .unwrap();
    kernel
        .sys_verify_agent_image(supervisor, authority, image)
        .unwrap();
    kernel
        .sys_launch_task_agent(
            supervisor,
            task_authority,
            task,
            image,
            AgentEntryKind::Supervisor,
        )
        .unwrap();
    kernel.sys_accept_task(supervisor, task).unwrap();
    run(&mut kernel, supervisor, task);
    let first = kernel
        .sys_fault_task(supervisor, task, FaultKind::ExecutionTrap, 7)
        .unwrap();
    kernel
        .sys_recover_faulted_task(supervisor, authority, task)
        .unwrap();

    let receipt = kernel
        .sys_compact_fault_prefix(supervisor, authority, first)
        .expect("facade routes Fault compaction");
    assert_eq!(receipt.first(), first);
    assert_eq!(receipt.through(), first);
    assert_eq!(receipt.count(), 1);
    assert!(kernel.faults().is_empty());
    assert_eq!(kernel.tasks()[0].last_fault, None);

    run(&mut kernel, supervisor, task);
    assert_eq!(
        kernel
            .sys_fault_task(supervisor, task, FaultKind::ExecutionTrap, 8)
            .unwrap(),
        FaultId::new(2)
    );
}

fn run(kernel: &mut TestKernel, agent: AgentId, task: agent_kernel_core::TaskId) {
    kernel.sys_enqueue_task(agent, task).unwrap();
    kernel.sys_dispatch_next_with_quantum(agent, 2).unwrap();
}
