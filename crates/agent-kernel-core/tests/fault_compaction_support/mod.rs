use agent_kernel_core::{
    AgentEntryKind, AgentId, AgentImageDigest, AgentImageKind, CapabilityId, FaultId, FaultKind,
    IntentKind, KernelCore, Operation, OperationSet, ResourceId, ResourceKind, TaskId,
    VerificationRequirement,
};

pub type TestCore<const EVENTS: usize, const MESSAGES: usize, const FAULTS: usize> =
    KernelCore<1, 2, 8, EVENTS, 0, 0, 0, 1, 1, 1, MESSAGES, 0, 0, FAULTS, 0, 0, 0, 1>;

#[derive(Copy, Clone)]
pub struct Fixture {
    pub actor: AgentId,
    pub root: ResourceId,
    pub root_authority: CapabilityId,
    pub resource: ResourceId,
    pub authority: CapabilityId,
    pub task_authority: CapabilityId,
    pub task: TaskId,
}

pub fn all_operations() -> OperationSet {
    OperationSet::only(Operation::Observe)
        .with(Operation::Act)
        .with(Operation::Verify)
        .with(Operation::Rollback)
        .with(Operation::Delegate)
}

pub fn running_fixture<const EVENTS: usize, const MESSAGES: usize, const FAULTS: usize>(
    kind: AgentEntryKind,
    child_resource: bool,
) -> (TestCore<EVENTS, MESSAGES, FAULTS>, Fixture) {
    let mut core = TestCore::new();
    let actor = AgentId::new(1);
    core.register_agent(actor).expect("actor registers");
    let root = core
        .register_resource(ResourceKind::Workspace, None)
        .expect("root resource registers");
    let root_authority = core
        .grant_capability(actor, root, all_operations())
        .expect("root authority grants");
    let (resource, authority) = if child_resource {
        let child = core
            .create_resource(
                actor,
                ResourceKind::Service,
                Some((root, root_authority)),
                all_operations(),
            )
            .expect("child resource creates");
        (child.resource, child.capability)
    } else {
        (root, root_authority)
    };
    let intent = core
        .declare_intent(
            actor,
            authority,
            resource,
            IntentKind::Act,
            VerificationRequirement::Required,
        )
        .expect("intent declares");
    let task = core
        .create_task(actor, authority, intent)
        .expect("task creates");
    core.delegate_task(actor, authority, task, actor)
        .expect("task delegates");
    let task_authority = core.tasks()[0]
        .delegated_capability
        .expect("task authority exists");
    let image_kind = if kind == AgentEntryKind::Supervisor {
        AgentImageKind::Supervisor
    } else {
        AgentImageKind::Worker
    };
    let image = core
        .register_agent_image(
            actor,
            authority,
            resource,
            image_kind,
            AgentImageDigest::new([0x39; 32]),
            1,
            1,
        )
        .expect("image registers");
    core.verify_agent_image(actor, authority, image)
        .expect("image verifies");
    core.launch_task_agent(actor, task_authority, task, image, kind)
        .expect("actor launches");
    core.accept_task(actor, task).expect("task accepts");
    dispatch(&mut core, actor, task);
    (
        core,
        Fixture {
            actor,
            root,
            root_authority,
            resource,
            authority,
            task_authority,
            task,
        },
    )
}

pub fn dispatch<const EVENTS: usize, const MESSAGES: usize, const FAULTS: usize>(
    core: &mut TestCore<EVENTS, MESSAGES, FAULTS>,
    actor: AgentId,
    task: TaskId,
) {
    core.enqueue_task(actor, task).expect("task enqueues");
    core.dispatch_next_with_quantum(actor, 2)
        .expect("task dispatches");
}

pub fn fault<const EVENTS: usize, const MESSAGES: usize, const FAULTS: usize>(
    core: &mut TestCore<EVENTS, MESSAGES, FAULTS>,
    fixture: Fixture,
    detail: u64,
) -> FaultId {
    core.fault_task(
        fixture.actor,
        fixture.task,
        FaultKind::ExecutionTrap,
        detail,
    )
    .expect("running task faults")
}

pub fn recover<const EVENTS: usize, const MESSAGES: usize, const FAULTS: usize>(
    core: &mut TestCore<EVENTS, MESSAGES, FAULTS>,
    fixture: Fixture,
) {
    core.recover_faulted_task(fixture.actor, fixture.authority, fixture.task)
        .expect("faulted task recovers");
}
