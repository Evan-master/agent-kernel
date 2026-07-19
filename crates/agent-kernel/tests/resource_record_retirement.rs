use agent_kernel::AgentKernel;
use agent_kernel_core::{
    AgentEntryKind, AgentId, AgentImageDigest, AgentImageKind, EventKind, KernelError, Operation,
    OperationSet, ResourceId, ResourceKind,
};

type TestKernel = AgentKernel<2, 3, 4, 32, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 2, 0, 0, 0, 0, 0>;

#[test]
fn facade_reclaims_a_retired_resource_record_and_preserves_fresh_ids() {
    let mut kernel = TestKernel::new();
    let actor = AgentId::new(1);
    kernel.sys_register_agent(actor).unwrap();
    let root = kernel
        .sys_register_resource(ResourceKind::Workspace, None)
        .unwrap();
    let operations = OperationSet::only(Operation::Act)
        .with(Operation::Verify)
        .with(Operation::Rollback)
        .with(Operation::Delegate);
    let authority = kernel.sys_grant(actor, root, operations).unwrap();
    let image = kernel
        .sys_register_agent_image(
            actor,
            authority,
            root,
            AgentImageKind::Supervisor,
            AgentImageDigest::new([0x52; 32]),
            1,
            1,
        )
        .unwrap();
    kernel
        .sys_verify_agent_image(actor, authority, image)
        .unwrap();
    kernel
        .sys_launch_agent(
            actor,
            authority,
            root,
            image,
            AgentEntryKind::Supervisor,
            None,
        )
        .unwrap();
    let target = kernel
        .sys_create_resource(
            actor,
            ResourceKind::Service,
            Some((root, authority)),
            OperationSet::only(Operation::Rollback),
        )
        .unwrap();
    kernel
        .sys_retire_resource(actor, target.capability, target.resource)
        .unwrap();
    let cleanup = kernel
        .sys_revoke_capability_for_cleanup(actor, authority, target.capability)
        .expect("facade cleanup revokes the residual capability");
    assert_eq!(cleanup.kind, EventKind::CapabilityRevoked);
    kernel
        .sys_compact_capability(actor, authority, target.capability)
        .unwrap();

    let receipt = kernel
        .sys_retire_resource_record(actor, authority, target.resource)
        .expect("facade retires the dense Resource record");

    assert_eq!(receipt.resource(), target.resource);
    assert_eq!(receipt.actor(), actor);
    assert_eq!(receipt.authority(), authority);
    assert_eq!(kernel.resources().len(), 1);
    assert_eq!(kernel.resources()[0].id, root);
    assert_eq!(
        kernel.capability(target.capability),
        Err(KernelError::CapabilityNotFound)
    );

    let fresh = kernel
        .sys_create_resource(
            actor,
            ResourceKind::Service,
            Some((root, authority)),
            OperationSet::only(Operation::Act),
        )
        .unwrap();
    assert_eq!(fresh.resource, ResourceId::new(3));
    assert!(fresh.capability.raw() > target.capability.raw());
}
