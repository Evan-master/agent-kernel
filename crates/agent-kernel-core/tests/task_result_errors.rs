use agent_kernel_core::{
    AgentId, CapabilityId, IntentKind, KernelCore, KernelError, Operation, OperationSet,
    ResourceKind, TaskId, TaskResult, VerificationRequirement,
};

type TestCore = KernelCore<2, 1, 3, 32, 0, 0, 0, 2, 2, 2>;

struct DelegatedTasks {
    assignee: AgentId,
    first: TaskId,
    first_capability: CapabilityId,
    second: TaskId,
    second_capability: CapabilityId,
}

fn delegated_tasks(core: &mut TestCore) -> DelegatedTasks {
    let owner = AgentId::new(3);
    let assignee = AgentId::new(4);
    core.register_agent(owner).unwrap();
    core.register_agent(assignee).unwrap();
    let resource = core
        .register_resource(ResourceKind::Workspace, None)
        .unwrap();
    let owner_capability = core
        .grant_capability(
            owner,
            resource,
            OperationSet::empty()
                .with(Operation::Act)
                .with(Operation::Delegate),
        )
        .unwrap();
    let mut create = || {
        let intent = core
            .declare_intent(
                owner,
                owner_capability,
                resource,
                IntentKind::Act,
                VerificationRequirement::Required,
            )
            .unwrap();
        let task = core.create_task(owner, owner_capability, intent).unwrap();
        let capability = core
            .delegate_task(owner, owner_capability, task, assignee)
            .unwrap()
            .capability
            .unwrap();
        core.accept_task(assignee, task).unwrap();
        (task, capability)
    };
    let (first, first_capability) = create();
    let (second, second_capability) = create();
    DelegatedTasks {
        assignee,
        first,
        first_capability,
        second,
        second_capability,
    }
}

#[test]
fn task_result_rejects_another_tasks_capability_before_status_mutation() {
    let mut core = TestCore::new();
    let tasks = delegated_tasks(&mut core);
    let events_before = core.events().len();

    let error = core.submit_task_result(
        tasks.assignee,
        tasks.first_capability,
        tasks.second,
        TaskResult { code: 1, value: 2 },
    );

    assert_eq!(error, Err(KernelError::CapabilityScopeMismatch));
    assert_eq!(core.tasks()[1].result, None);
    assert_eq!(core.events().len(), events_before);
}

#[test]
fn task_result_requires_running_status_without_mutation() {
    let mut core = TestCore::new();
    let tasks = delegated_tasks(&mut core);
    let events_before = core.events().len();

    let error = core.submit_task_result(
        tasks.assignee,
        tasks.second_capability,
        tasks.second,
        TaskResult { code: 3, value: 4 },
    );

    assert_eq!(error, Err(KernelError::TaskStatusMismatch));
    assert_eq!(core.tasks()[1].result, None);
    assert_eq!(core.events().len(), events_before);
    assert_ne!(tasks.first, tasks.second);
}
