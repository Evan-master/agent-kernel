use agent_kernel_core::{
    AgentEntryKind, AgentId, AgentImageDigest, AgentImageId, AgentImageKind, CapabilityId,
    KernelCore, Operation, OperationSet, ResourceId, ResourceKind, TaskId,
};

pub type TestCore<const EVENTS: usize> = CapacityCore<EVENTS, 2>;
pub type CapacityCore<const EVENTS: usize, const RUNTIME_ADMISSIONS: usize> = KernelCore<
    3,
    2,
    8,
    EVENTS,
    0,
    64,
    0,
    2,
    2,
    2,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    3,
    0,
    0,
    0,
    0,
    RUNTIME_ADMISSIONS,
>;

#[derive(Copy, Clone)]
pub struct Fixture {
    pub supervisor: AgentId,
    pub target: AgentId,
    pub authority: CapabilityId,
    pub resource: ResourceId,
    pub task: TaskId,
    pub task_capability: CapabilityId,
    pub image: AgentImageId,
}

#[derive(Copy, Clone)]
pub struct SecondTarget {
    pub target: AgentId,
    pub task: TaskId,
    pub task_capability: CapabilityId,
}

#[derive(Copy, Clone)]
pub struct PairFixture {
    pub first: Fixture,
    pub second: SecondTarget,
}

pub fn prepared<const EVENTS: usize>() -> (TestCore<EVENTS>, Fixture) {
    prepared_with_capacity::<EVENTS, 2>()
}

pub fn prepared_with_capacity<const EVENTS: usize, const RUNTIME_ADMISSIONS: usize>(
) -> (CapacityCore<EVENTS, RUNTIME_ADMISSIONS>, Fixture) {
    let mut core = CapacityCore::<EVENTS, RUNTIME_ADMISSIONS>::new();
    let supervisor = AgentId::new(1);
    let target = AgentId::new(2);
    core.register_agent(supervisor)
        .expect("supervisor registers");
    core.register_agent(target).expect("target registers");
    let resource = core
        .register_resource(ResourceKind::Workspace, None)
        .expect("resource fits");
    let authority = core
        .grant_capability(
            supervisor,
            resource,
            OperationSet::only(Operation::Act)
                .with(Operation::Delegate)
                .with(Operation::Verify)
                .with(Operation::Rollback)
                .with(Operation::Observe),
        )
        .expect("authority fits");
    let supervisor_image = core
        .register_agent_image(
            supervisor,
            authority,
            resource,
            AgentImageKind::Supervisor,
            AgentImageDigest::new([1; 32]),
            1,
            1,
        )
        .expect("supervisor image registers");
    core.verify_agent_image(supervisor, authority, supervisor_image)
        .expect("supervisor image verifies");
    core.launch_agent(
        supervisor,
        authority,
        resource,
        supervisor_image,
        AgentEntryKind::Supervisor,
        None,
    )
    .expect("supervisor launches");

    let intent = core
        .declare_intent(
            supervisor,
            authority,
            resource,
            agent_kernel_core::IntentKind::Act,
            agent_kernel_core::VerificationRequirement::Required,
        )
        .expect("intent declares");
    let task = core
        .create_task(supervisor, authority, intent)
        .expect("task creates");
    core.delegate_task(supervisor, authority, task, target)
        .expect("task delegates");
    let task_capability = core.tasks()[0]
        .delegated_capability
        .expect("task capability exists");
    let image = core
        .register_agent_image(
            supervisor,
            authority,
            resource,
            AgentImageKind::Worker,
            AgentImageDigest::new([2; 32]),
            1,
            1,
        )
        .expect("worker image registers");
    core.verify_agent_image(supervisor, authority, image)
        .expect("worker image verifies");
    core.launch_task_agent(target, task_capability, task, image, AgentEntryKind::Worker)
        .expect("worker launches");
    core.accept_task(target, task).expect("worker accepts");

    (
        core,
        Fixture {
            supervisor,
            target,
            authority,
            resource,
            task,
            task_capability,
            image,
        },
    )
}

pub fn prepared_pair<const EVENTS: usize>() -> (TestCore<EVENTS>, PairFixture) {
    let (mut core, first) = prepared::<EVENTS>();
    let target = AgentId::new(3);
    core.register_agent(target)
        .expect("second target registers");
    let intent = core
        .declare_intent(
            first.supervisor,
            first.authority,
            first.resource,
            agent_kernel_core::IntentKind::Act,
            agent_kernel_core::VerificationRequirement::Required,
        )
        .expect("second intent declares");
    let task = core
        .create_task(first.supervisor, first.authority, intent)
        .expect("second task creates");
    core.delegate_task(first.supervisor, first.authority, task, target)
        .expect("second task delegates");
    let task_capability = core.tasks()[1]
        .delegated_capability
        .expect("second task capability exists");
    let image = core
        .register_agent_image(
            first.supervisor,
            first.authority,
            first.resource,
            AgentImageKind::Worker,
            AgentImageDigest::new([3; 32]),
            1,
            1,
        )
        .expect("second worker image registers");
    core.verify_agent_image(first.supervisor, first.authority, image)
        .expect("second worker image verifies");
    core.launch_task_agent(target, task_capability, task, image, AgentEntryKind::Worker)
        .expect("second worker launches");
    core.accept_task(target, task)
        .expect("second worker accepts");

    (
        core,
        PairFixture {
            first,
            second: SecondTarget {
                target,
                task,
                task_capability,
            },
        },
    )
}
