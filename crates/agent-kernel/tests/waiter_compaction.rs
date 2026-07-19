use agent_kernel::AgentKernel;
use agent_kernel_core::{
    AgentEntryKind, AgentId, AgentImageDigest, AgentImageKind, IntentKind, Operation, OperationSet,
    ResourceKind, SignalKey, VerificationRequirement, WaiterId,
};

type TestKernel = AgentKernel<1, 1, 3, 48, 0, 0, 0, 1, 1, 1, 0, 0, 0, 0, 0, 0, 1, 1>;

#[test]
fn facade_compacts_inactive_waiter_and_reuses_slot_with_fresh_identity() {
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
            AgentImageDigest::new([0x38; 32]),
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
    kernel.sys_enqueue_task(supervisor, task).unwrap();
    kernel
        .sys_dispatch_next_with_quantum(supervisor, 2)
        .unwrap();
    let first_signal = SignalKey::new(7);
    let first = kernel
        .sys_wait_task(supervisor, task_authority, task, resource, first_signal)
        .unwrap();
    kernel
        .sys_emit_signal(supervisor, authority, resource, first_signal)
        .unwrap();

    let receipt = kernel
        .sys_compact_waiter_prefix(supervisor, authority, first)
        .expect("facade routes Waiter compaction");
    assert_eq!(receipt.first(), first);
    assert_eq!(receipt.through(), first);
    assert_eq!(receipt.count(), 1);
    assert!(kernel.waiters().is_empty());

    kernel
        .sys_dispatch_next_with_quantum(supervisor, 2)
        .unwrap();
    let second = kernel
        .sys_wait_task(
            supervisor,
            task_authority,
            task,
            resource,
            SignalKey::new(8),
        )
        .expect("returned physical slot accepts a fresh waiter");
    assert_eq!(second, WaiterId::new(2));
    assert_eq!(kernel.waiters()[0].id, second);
}
