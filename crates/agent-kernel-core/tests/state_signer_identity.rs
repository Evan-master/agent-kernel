use agent_kernel_core::{
    AgentEntryKind, AgentId, AgentImageDigest, AgentImageKind, KernelCore, KernelError, Operation,
    OperationSet, ResourceKind,
};

type TestCore = KernelCore<1, 1, 1, 8, 0, 0, 0, 0, 0, 0>;

#[test]
fn state_signer_image_launches_only_as_state_signer_entry() {
    let mut core = TestCore::new();
    let signer = AgentId::new(1);
    core.register_agent(signer).unwrap();
    let resource = core
        .register_resource(ResourceKind::Workspace, None)
        .unwrap();
    let capability = core
        .grant_capability(
            signer,
            resource,
            OperationSet::only(Operation::Act).with(Operation::Verify),
        )
        .unwrap();
    let image = core
        .register_agent_image(
            signer,
            capability,
            resource,
            AgentImageKind::StateSigner,
            AgentImageDigest::new([0x53; 32]),
            1,
            1,
        )
        .unwrap();
    core.verify_agent_image(signer, capability, image).unwrap();

    assert_eq!(
        core.launch_agent(
            signer,
            capability,
            resource,
            image,
            AgentEntryKind::Supervisor,
            None,
        ),
        Err(KernelError::AgentImageKindMismatch)
    );
    core.launch_agent(
        signer,
        capability,
        resource,
        image,
        AgentEntryKind::StateSigner,
        None,
    )
    .unwrap();
    assert_eq!(
        core.agent_image(image).unwrap().kind,
        AgentImageKind::StateSigner
    );
    assert_eq!(
        core.agent_entry(signer).unwrap().kind,
        AgentEntryKind::StateSigner
    );
}
