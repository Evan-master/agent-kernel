use agent_kernel_core::{
    AgentEntryKind, AgentId, AgentImageDigest, AgentImageKind, CapabilityId, EventKind, IntentKind,
    IntentStatus, KernelCore, Operation, OperationSet, ResourceId, ResourceKind,
    VerificationRequirement,
};

type TestCore = KernelCore<2, 4, 8, 16, 2, 2, 2, 4, 4, 4, 0, 0, 0, 0, 0, 0, 0, 2>;

fn prepare_agent(core: &mut TestCore) -> (AgentId, CapabilityId, ResourceId) {
    let agent = AgentId::new(1);
    core.register_agent(agent).expect("agent should register");
    let resource = core
        .register_resource(ResourceKind::Workspace, None)
        .expect("resource should fit");
    let capability = core
        .grant_capability(
            agent,
            resource,
            OperationSet::empty()
                .with(Operation::Act)
                .with(Operation::Verify),
        )
        .expect("capability should fit");

    (agent, capability, resource)
}

fn digest(byte: u8) -> AgentImageDigest {
    AgentImageDigest::new([byte; 32])
}

#[test]
fn launch_agent_records_entry_and_event() {
    let mut core = TestCore::new();
    let (agent, capability, resource) = prepare_agent(&mut core);
    let image = core
        .register_agent_image(
            agent,
            capability,
            resource,
            AgentImageKind::Supervisor,
            digest(1),
            1,
            1,
        )
        .expect("image should register");
    core.verify_agent_image(agent, capability, image)
        .expect("image should verify");

    let event = core
        .launch_agent(
            agent,
            capability,
            resource,
            image,
            AgentEntryKind::Supervisor,
            None,
        )
        .expect("agent should launch");

    assert_eq!(event.kind, EventKind::AgentLaunched);
    assert_eq!(event.agent, agent);
    assert_eq!(event.resource, Some(resource));
    assert_eq!(event.capability, Some(capability));
    assert_eq!(event.intent, None);
    assert_eq!(event.agent_image, Some(image));
    assert_eq!(event.target_agent, Some(agent));
    assert_eq!(core.events()[4], event);

    assert_eq!(core.agent_entries().len(), 1);
    let entry = core.agent_entry(agent).expect("agent entry should exist");
    assert_eq!(entry.agent, agent);
    assert_eq!(entry.resource, resource);
    assert_eq!(entry.capability, capability);
    assert_eq!(entry.image, image);
    assert_eq!(entry.kind, AgentEntryKind::Supervisor);
    assert_eq!(entry.intent, None);
}

#[test]
fn fault_handler_image_launches_only_as_first_class_fault_handler_entry() {
    let mut core = TestCore::new();
    let (agent, capability, resource) = prepare_agent(&mut core);
    let image = core
        .register_agent_image(
            agent,
            capability,
            resource,
            AgentImageKind::FaultHandler,
            digest(0xf0),
            1,
            1,
        )
        .expect("fault handler image should register");
    core.verify_agent_image(agent, capability, image)
        .expect("fault handler image should verify");

    core.launch_agent(
        agent,
        capability,
        resource,
        image,
        AgentEntryKind::FaultHandler,
        None,
    )
    .expect("matching fault handler entry should launch");

    assert_eq!(
        core.agent_entry(agent).expect("entry should exist").kind,
        AgentEntryKind::FaultHandler
    );
}

#[test]
fn launch_agent_accepts_declared_action_intent() {
    let mut core = TestCore::new();
    let (agent, capability, resource) = prepare_agent(&mut core);
    let image = core
        .register_agent_image(
            agent,
            capability,
            resource,
            AgentImageKind::Worker,
            digest(2),
            1,
            1,
        )
        .expect("image should register");
    core.verify_agent_image(agent, capability, image)
        .expect("image should verify");
    let intent = core
        .declare_intent(
            agent,
            capability,
            resource,
            IntentKind::Act,
            VerificationRequirement::Required,
        )
        .expect("intent should declare");

    let event = core
        .launch_agent(
            agent,
            capability,
            resource,
            image,
            AgentEntryKind::Worker,
            Some(intent),
        )
        .expect("agent should launch with intent");
    let entry = core.agent_entry(agent).expect("agent entry should exist");

    assert_eq!(entry.intent, Some(intent));
    assert_eq!(event.intent, Some(intent));
    assert_eq!(core.intents()[0].status, IntentStatus::Declared);
}
