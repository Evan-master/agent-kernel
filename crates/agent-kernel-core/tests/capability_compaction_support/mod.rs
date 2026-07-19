use agent_kernel_core::{
    AgentEntryKind, AgentId, AgentImageDigest, AgentImageId, AgentImageKind, CapabilityId,
    IntentKind, KernelCore, Operation, OperationSet, ResourceId, ResourceKind, TaskId,
    VerificationRequirement,
};

pub(super) type TestCore<const EVENTS: usize> =
    KernelCore<4, 4, 10, EVENTS, 2, EVENTS, 2, 4, 4, 4, 2, 0, 2, 0, 0, 0, 2, 4, 0, 0, 0, 0, 4>;

#[derive(Copy, Clone)]
pub(super) struct Fixture {
    pub supervisor: AgentId,
    pub worker: AgentId,
    pub other: AgentId,
    pub authority: CapabilityId,
    pub resource: ResourceId,
}

pub(super) fn prepared<const EVENTS: usize>() -> (TestCore<EVENTS>, Fixture) {
    let mut core = TestCore::new();
    let supervisor = AgentId::new(1);
    let worker = AgentId::new(2);
    let other = AgentId::new(3);
    for agent in [supervisor, worker, other] {
        core.register_agent(agent).expect("agent registers");
    }
    let resource = core
        .register_resource(ResourceKind::Workspace, None)
        .expect("root resource registers");
    let authority = core
        .grant_capability(supervisor, resource, all_operations())
        .expect("root authority grants");
    let image = register_image(
        &mut core,
        supervisor,
        authority,
        resource,
        AgentImageKind::Supervisor,
        1,
    );
    core.launch_agent(
        supervisor,
        authority,
        resource,
        image,
        AgentEntryKind::Supervisor,
        None,
    )
    .expect("supervisor launches");

    (
        core,
        Fixture {
            supervisor,
            worker,
            other,
            authority,
            resource,
        },
    )
}

pub(super) fn all_operations() -> OperationSet {
    OperationSet::only(Operation::Observe)
        .with(Operation::Act)
        .with(Operation::Verify)
        .with(Operation::Checkpoint)
        .with(Operation::Rollback)
        .with(Operation::Delegate)
}

pub(super) fn register_image<const EVENTS: usize>(
    core: &mut TestCore<EVENTS>,
    actor: AgentId,
    authority: CapabilityId,
    resource: ResourceId,
    kind: AgentImageKind,
    digest_byte: u8,
) -> AgentImageId {
    let image = core
        .register_agent_image(
            actor,
            authority,
            resource,
            kind,
            AgentImageDigest::new([digest_byte; 32]),
            1,
            1,
        )
        .expect("image registers");
    core.verify_agent_image(actor, authority, image)
        .expect("image verifies");
    image
}

pub(super) fn cancelled_delegated_task<const EVENTS: usize>(
    core: &mut TestCore<EVENTS>,
    fixture: Fixture,
) -> (TaskId, CapabilityId) {
    let intent = core
        .declare_intent(
            fixture.supervisor,
            fixture.authority,
            fixture.resource,
            IntentKind::Act,
            VerificationRequirement::Required,
        )
        .expect("intent declares");
    let task = core
        .create_task(fixture.supervisor, fixture.authority, intent)
        .expect("task creates");
    core.delegate_task(fixture.supervisor, fixture.authority, task, fixture.worker)
        .expect("task delegates");
    let capability = core
        .task(task)
        .expect("task exists")
        .delegated_capability
        .expect("task capability exists");
    core.cancel_task(fixture.supervisor, fixture.authority, task)
        .expect("task cancels");
    (task, capability)
}
