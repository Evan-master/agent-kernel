use agent_kernel::AgentKernel;
use agent_kernel_core::{
    AgentEntryKind, AgentId, AgentImageDigest, AgentImageKind, CapabilityId, KernelError,
    Operation, OperationSet, ResourceKind,
};

type TestKernel = AgentKernel<2, 1, 3, 32, 0, 0, 0, 0, 0, 0>;

#[test]
fn facade_forwards_capability_inspection_compaction_and_slot_reuse() {
    let mut kernel = TestKernel::new();
    let supervisor = AgentId::new(1);
    let worker = AgentId::new(2);
    kernel.sys_register_agent(supervisor).unwrap();
    kernel.sys_register_agent(worker).unwrap();
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
    let image = kernel
        .sys_register_agent_image(
            supervisor,
            authority,
            resource,
            AgentImageKind::Supervisor,
            AgentImageDigest::new([7; 32]),
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
    let target = kernel
        .sys_derive_capability(
            supervisor,
            authority,
            worker,
            OperationSet::only(Operation::Rollback),
        )
        .unwrap();
    kernel
        .sys_revoke_derived_capability(supervisor, authority, target)
        .unwrap();

    assert_eq!(kernel.capability_capacity(), 3);
    assert_eq!(kernel.capability_count(), 2);
    assert_eq!(kernel.capability(target).unwrap().id, target);
    let receipt = kernel
        .sys_compact_capability(supervisor, authority, target)
        .unwrap();
    assert_eq!(receipt.capability(), target);
    assert_eq!(kernel.capability_count(), 1);
    assert_eq!(
        kernel.capability(target),
        Err(KernelError::CapabilityNotFound)
    );

    let replacement = kernel
        .sys_derive_capability(
            supervisor,
            authority,
            worker,
            OperationSet::only(Operation::Rollback),
        )
        .unwrap();
    assert_eq!(replacement, CapabilityId::new(3));
    assert_eq!(kernel.capability_count(), 2);
}
