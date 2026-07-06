use agent_kernel::AgentKernel;
use agent_kernel_core::{
    AgentEntryKind, AgentId, AgentImageDigest, AgentImageKind, EventKind, Operation, OperationSet,
    ResourceKind,
};

#[test]
fn facade_launches_agent_and_exposes_entry() {
    let mut kernel = AgentKernel::<2, 4, 8, 16, 1, 1, 1, 2, 2, 2>::new();
    let agent = AgentId::new(1);

    kernel
        .sys_register_agent(agent)
        .expect("agent should register");
    let resource = kernel
        .sys_register_resource(ResourceKind::Workspace, None)
        .expect("resource should fit");
    let capability = kernel
        .sys_grant(agent, resource, OperationSet::only(Operation::Act))
        .expect("capability should fit");
    let image_digest = AgentImageDigest::new([11; 32]);
    let image = kernel
        .sys_register_agent_image(
            agent,
            capability,
            resource,
            AgentImageKind::Supervisor,
            image_digest,
            1,
            1,
        )
        .expect("image should register through facade");

    let event = kernel
        .sys_launch_agent(
            agent,
            capability,
            resource,
            image,
            AgentEntryKind::Supervisor,
            None,
        )
        .expect("agent should launch");
    let entry = kernel.agent_entry(agent).expect("agent entry should exist");
    let image_record = kernel
        .agent_image(image)
        .expect("image should be queryable");

    assert_eq!(event.kind, EventKind::AgentLaunched);
    assert_eq!(event.agent_image, Some(image));
    assert_eq!(kernel.agent_entries().len(), 1);
    assert_eq!(kernel.agent_images().len(), 1);
    assert_eq!(entry.agent, agent);
    assert_eq!(entry.resource, resource);
    assert_eq!(entry.capability, capability);
    assert_eq!(entry.image, image);
    assert_eq!(entry.kind, AgentEntryKind::Supervisor);
    assert_eq!(entry.intent, None);
    assert_eq!(image_record.digest, image_digest);
}
