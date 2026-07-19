use agent_kernel::AgentKernel;
use agent_kernel_core::{
    AgentId, AgentImageDigest, AgentImageKind, EventKind, KernelError, Operation, OperationSet,
    ResourceKind,
};

type TestKernel = AgentKernel<2, 1, 2, 32, 0, 0, 0, 0, 0, 0>;

#[test]
fn facade_retires_image_record_and_reuses_capacity_with_monotonic_identity() {
    let mut kernel = TestKernel::new();
    let actor = AgentId::new(1);
    kernel.sys_register_agent(actor).unwrap();
    let resource = kernel
        .sys_register_resource(ResourceKind::Workspace, None)
        .unwrap();
    let authority = kernel
        .sys_grant(
            actor,
            resource,
            OperationSet::only(Operation::Act).with(Operation::Rollback),
        )
        .unwrap();
    let retained = kernel
        .sys_register_agent_image(
            actor,
            authority,
            resource,
            AgentImageKind::Worker,
            AgentImageDigest::new([1; 32]),
            1,
            1,
        )
        .unwrap();
    let target = kernel
        .sys_register_agent_image(
            actor,
            authority,
            resource,
            AgentImageKind::Worker,
            AgentImageDigest::new([2; 32]),
            1,
            1,
        )
        .unwrap();
    kernel
        .sys_retire_agent_image(actor, authority, target)
        .unwrap();

    let retirement = kernel
        .sys_retire_agent_image_record(actor, authority, target)
        .expect("facade routes image record retirement");
    assert_eq!(retirement.image(), target);
    assert_eq!(retirement.record().resource, resource);
    assert_eq!(
        kernel.agent_image(target),
        Err(KernelError::AgentImageNotFound)
    );

    let fresh = kernel
        .sys_register_agent_image(
            actor,
            authority,
            resource,
            AgentImageKind::Worker,
            AgentImageDigest::new([3; 32]),
            1,
            1,
        )
        .expect("fresh identity reuses image slot");
    assert_eq!(fresh.raw(), 3);
    assert_eq!(
        kernel
            .agent_images()
            .iter()
            .map(|record| record.id)
            .collect::<Vec<_>>(),
        vec![retained, fresh]
    );
    assert_eq!(
        kernel.events().last().map(|event| event.kind),
        Some(EventKind::AgentImageRegistered)
    );
}
