use agent_kernel::AgentKernel;
use agent_kernel_core::{
    AgentEntryKind, AgentId, AgentImageDigest, AgentImageKind, IntentKind, KernelError, Operation,
    OperationSet, ResourceKind, VerificationRequirement,
};

type TestKernel = AgentKernel<1, 1, 1, 32, 0, 0, 0, 1, 1, 0>;

#[test]
fn facade_forwards_intent_capacity_lookup_and_compaction() {
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
                .with(Operation::Rollback)
                .with(Operation::Verify),
        )
        .unwrap();
    let image = kernel
        .sys_register_agent_image(
            supervisor,
            authority,
            resource,
            AgentImageKind::Supervisor,
            AgentImageDigest::new([9; 32]),
            1,
            1,
        )
        .unwrap();
    kernel
        .sys_verify_agent_image(supervisor, authority, image)
        .unwrap();
    kernel
        .sys_launch_agent(
            supervisor,
            authority,
            resource,
            image,
            AgentEntryKind::Supervisor,
            None,
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
    kernel.sys_cancel_task(supervisor, authority, task).unwrap();
    kernel
        .sys_compact_task_prefix(supervisor, authority, task)
        .unwrap();

    assert_eq!(kernel.intent_capacity(), 1);
    assert_eq!(kernel.intent(intent).unwrap().id, intent);
    let receipt = kernel
        .sys_compact_intent_prefix(supervisor, authority, intent)
        .unwrap();
    assert_eq!(receipt.first(), intent);
    assert_eq!(receipt.through(), intent);
    assert_eq!(receipt.count(), 1);
    assert!(kernel.intents().is_empty());
    assert_eq!(kernel.intent(intent), Err(KernelError::IntentNotFound));
}
