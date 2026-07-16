use agent_kernel_core::{
    AgentEntryKind, AgentId, AgentImageDigest, AgentImageKind, KernelCore, KernelError, Operation,
    OperationSet, ResourceKind,
};

type TestCore = KernelCore<1, 1, 1, 8, 0, 0, 0, 0, 0, 0>;

#[test]
fn verifier_image_launches_only_as_verifier_entry() {
    let mut core = TestCore::new();
    let verifier = AgentId::new(1);
    core.register_agent(verifier).unwrap();
    let resource = core
        .register_resource(ResourceKind::Workspace, None)
        .unwrap();
    let capability = core
        .grant_capability(
            verifier,
            resource,
            OperationSet::empty()
                .with(Operation::Act)
                .with(Operation::Verify),
        )
        .unwrap();
    let image = core
        .register_agent_image(
            verifier,
            capability,
            resource,
            AgentImageKind::Verifier,
            AgentImageDigest::new([0x33; 32]),
            1,
            1,
        )
        .unwrap();
    core.verify_agent_image(verifier, capability, image)
        .unwrap();

    assert_eq!(
        core.launch_agent(
            verifier,
            capability,
            resource,
            image,
            AgentEntryKind::Worker,
            None,
        ),
        Err(KernelError::AgentImageKindMismatch)
    );
    core.launch_agent(
        verifier,
        capability,
        resource,
        image,
        AgentEntryKind::Verifier,
        None,
    )
    .unwrap();
    assert_eq!(
        core.agent_image(image).unwrap().kind,
        AgentImageKind::Verifier
    );
    assert_eq!(
        core.agent_entry(verifier).unwrap().kind,
        AgentEntryKind::Verifier
    );
}
