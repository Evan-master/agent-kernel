use agent_kernel::AgentKernel;
use agent_kernel_core::{
    agent_image_signer_id, AgentId, AgentImageKind, AgentImageKindScope, AgentImageSignerStatus,
    EventKind, Operation, OperationSet, ResourceKind,
};

type Kernel = AgentKernel<2, 2, 4, 16, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 2>;

#[test]
fn facade_exposes_generation_guarded_signer_rotation() {
    let mut kernel = Kernel::new();
    let actor = AgentId::new(1);
    kernel
        .sys_register_agent(actor)
        .expect("actor should register");
    let resource = kernel
        .sys_register_resource(ResourceKind::Workspace, None)
        .expect("resource should register");
    let authority = kernel
        .sys_grant(
            actor,
            resource,
            OperationSet::only(Operation::Verify).with(Operation::Rollback),
        )
        .expect("authority should fit");
    let initial_key = [0x61; 32];
    let initial = kernel
        .sys_trust_agent_image_signer(
            actor,
            authority,
            resource,
            initial_key,
            AgentImageKindScope::only(AgentImageKind::Supervisor),
            1,
            1,
        )
        .expect("initial trust should cross the facade");
    let replacement_key = [0x62; 32];

    let rotation = kernel
        .sys_rotate_agent_image_signer(
            actor,
            authority,
            resource,
            1,
            initial.signer_id,
            replacement_key,
            AgentImageKindScope::only(AgentImageKind::Worker),
            1,
            1,
        )
        .expect("rotation should cross the facade");

    assert_eq!(kernel.agent_image_signer_policy_generation(), 2);
    assert_eq!(kernel.agent_image_signers().len(), 2);
    assert_eq!(rotation.previous().status, AgentImageSignerStatus::Revoked);
    assert_eq!(
        rotation.replacement().status,
        AgentImageSignerStatus::Active
    );
    assert_eq!(
        rotation.replacement().signer_id,
        agent_image_signer_id(replacement_key)
    );
    assert_eq!(
        &kernel.events()[kernel.events().len() - 2..]
            .iter()
            .map(|event| event.kind)
            .collect::<std::vec::Vec<_>>(),
        &[
            EventKind::AgentImageSignerTrusted,
            EventKind::AgentImageSignerRevoked,
        ]
    );
}
