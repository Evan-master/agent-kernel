use agent_kernel_core::{
    AgentEntryKind, AgentId, AgentImageDigest, AgentImageKind, CapabilityId, IntentKind,
    KernelCore, Operation, OperationSet, ResourceKind, TaskId, TaskResult, VerificationRequirement,
};

pub type TestCore<const EVENTS: usize> = KernelCore<3, 1, 4, EVENTS, 0, 0, 0, 2, 2, 2>;

pub const RESULT: TaskResult = TaskResult {
    code: 0x0a01,
    value: 0xa11c_0001,
};

#[allow(dead_code)]
pub struct InspectionFixture {
    pub verifier: AgentId,
    pub target: TaskId,
    pub verifier_task_capability: CapabilityId,
    pub verify_capability: CapabilityId,
}

pub fn setup<const EVENTS: usize>(
    core: &mut TestCore<EVENTS>,
    submit_result: bool,
    complete_target: bool,
    verifier_kind: AgentImageKind,
    entry_kind: AgentEntryKind,
    dispatch_verifier: bool,
) -> InspectionFixture {
    let owner = AgentId::new(1);
    let worker = AgentId::new(2);
    let verifier = AgentId::new(3);
    core.register_agent(owner).unwrap();
    core.register_agent(worker).unwrap();
    core.register_agent(verifier).unwrap();
    let resource = core
        .register_resource(ResourceKind::Workspace, None)
        .unwrap();
    let owner_capability = core
        .grant_capability(
            owner,
            resource,
            OperationSet::empty()
                .with(Operation::Act)
                .with(Operation::Delegate)
                .with(Operation::Verify),
        )
        .unwrap();

    let target_intent = core
        .declare_intent(
            owner,
            owner_capability,
            resource,
            IntentKind::Act,
            VerificationRequirement::Required,
        )
        .unwrap();
    let target = core
        .create_task(owner, owner_capability, target_intent)
        .unwrap();
    let target_capability = core
        .delegate_task(owner, owner_capability, target, worker)
        .unwrap()
        .capability
        .unwrap();
    let worker_image = core
        .register_agent_image(
            owner,
            owner_capability,
            resource,
            AgentImageKind::Worker,
            AgentImageDigest::new([0x11; 32]),
            1,
            1,
        )
        .unwrap();
    core.verify_agent_image(owner, owner_capability, worker_image)
        .unwrap();
    core.launch_task_agent(
        worker,
        target_capability,
        target,
        worker_image,
        AgentEntryKind::Worker,
    )
    .unwrap();
    core.accept_task(worker, target).unwrap();
    core.enqueue_task(worker, target).unwrap();
    core.dispatch_next(worker).unwrap();
    if submit_result {
        core.submit_task_result(worker, target_capability, target, RESULT)
            .unwrap();
    }
    if complete_target {
        core.complete_task(worker, target_capability, target)
            .unwrap();
    }

    let verifier_intent = core
        .declare_intent(
            owner,
            owner_capability,
            resource,
            IntentKind::Verify,
            VerificationRequirement::Optional,
        )
        .unwrap();
    let verifier_task = core
        .create_task(owner, owner_capability, verifier_intent)
        .unwrap();
    let verifier_task_capability = core
        .delegate_task(owner, owner_capability, verifier_task, verifier)
        .unwrap()
        .capability
        .unwrap();
    let verify_capability = core
        .derive_capability(
            owner,
            owner_capability,
            verifier,
            OperationSet::only(Operation::Verify),
        )
        .unwrap();
    let verifier_image = core
        .register_agent_image(
            owner,
            owner_capability,
            resource,
            verifier_kind,
            AgentImageDigest::new([0x22; 32]),
            1,
            1,
        )
        .unwrap();
    core.verify_agent_image(owner, owner_capability, verifier_image)
        .unwrap();
    core.launch_task_agent(
        verifier,
        verifier_task_capability,
        verifier_task,
        verifier_image,
        entry_kind,
    )
    .unwrap();
    core.accept_task(verifier, verifier_task).unwrap();
    if dispatch_verifier {
        core.enqueue_task(verifier, verifier_task).unwrap();
        core.dispatch_next(verifier).unwrap();
    }

    InspectionFixture {
        verifier,
        target,
        verifier_task_capability,
        verify_capability,
    }
}
