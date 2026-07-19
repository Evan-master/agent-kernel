use agent_kernel_core::{
    AgentEntryKind, AgentId, AgentImageDigest, AgentImageId, AgentImageKind, CapabilityId,
    IntentId, IntentKind, KernelCore, Operation, OperationSet, ResourceId, TaskId,
    VerificationRequirement,
};

pub(super) type TestCore<const EVENTS: usize> =
    KernelCore<6, 4, 16, EVENTS, 0, EVENTS, 0, 8, 8, 8, 8, 0, 0, 2, 2, 0, 4, 8, 2, 0, 0, 0, 4>;

#[derive(Copy, Clone)]
pub(super) struct Fixture {
    pub supervisor: AgentId,
    pub worker: AgentId,
    pub other: AgentId,
    pub resource: ResourceId,
    pub authority: CapabilityId,
}

#[derive(Copy, Clone)]
pub(super) struct TaskLaunch {
    pub agent: AgentId,
    pub intent: IntentId,
    pub task: TaskId,
    pub capability: CapabilityId,
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
        .register_resource(agent_kernel_core::ResourceKind::Workspace, None)
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
            resource,
            authority,
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
    owner: AgentId,
    authority: CapabilityId,
    resource: ResourceId,
    kind: AgentImageKind,
    digest_byte: u8,
) -> AgentImageId {
    let image = core
        .register_agent_image(
            owner,
            authority,
            resource,
            kind,
            AgentImageDigest::new([digest_byte; 32]),
            1,
            1,
        )
        .expect("image registers");
    core.verify_agent_image(owner, authority, image)
        .expect("image verifies");
    image
}

pub(super) fn launch_task<const EVENTS: usize>(
    core: &mut TestCore<EVENTS>,
    fixture: Fixture,
    agent: AgentId,
    resource: ResourceId,
    authority: CapabilityId,
    digest_byte: u8,
) -> TaskLaunch {
    let intent = core
        .declare_intent(
            fixture.supervisor,
            authority,
            resource,
            IntentKind::Act,
            VerificationRequirement::Required,
        )
        .expect("intent declares");
    let task = core
        .create_task(fixture.supervisor, authority, intent)
        .expect("task creates");
    core.delegate_task(fixture.supervisor, authority, task, agent)
        .expect("task delegates");
    let capability = core
        .task(task)
        .expect("task exists")
        .delegated_capability
        .expect("delegated capability exists");
    let image = register_image(
        core,
        fixture.supervisor,
        authority,
        resource,
        AgentImageKind::Worker,
        digest_byte,
    );
    core.launch_task_agent(agent, capability, task, image, AgentEntryKind::Worker)
        .expect("task agent launches");
    core.accept_task(agent, task).expect("task accepts");
    TaskLaunch {
        agent,
        intent,
        task,
        capability,
    }
}

pub(super) fn complete_and_verify<const EVENTS: usize>(
    core: &mut TestCore<EVENTS>,
    fixture: Fixture,
    launch: TaskLaunch,
    verification_authority: CapabilityId,
) {
    core.enqueue_task(launch.agent, launch.task)
        .expect("task enqueues");
    core.dispatch_next(launch.agent).expect("task dispatches");
    core.complete_task(launch.agent, launch.capability, launch.task)
        .expect("task completes");
    core.verify_task(fixture.supervisor, verification_authority, launch.task)
        .expect("task verifies");
}
