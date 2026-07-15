use agent_kernel_core::{
    AgentEntryKind, AgentExecutionState, AgentId, AgentImageDigest, AgentImageKind, CapabilityId,
    EventKind, IntentKind, KernelCore, KernelError, Operation, OperationSet, ResourceKind,
    TaskResult, TaskStatus, VerificationRequirement,
};

type TestCore<const EVENTS: usize> = KernelCore<2, 1, 2, EVENTS, 0, 0, 0, 1, 1, 1>;

#[derive(Copy, Clone)]
struct RunningTask {
    assignee: AgentId,
    task: agent_kernel_core::TaskId,
    capability: CapabilityId,
}

fn running_task<const EVENTS: usize>(core: &mut TestCore<EVENTS>) -> RunningTask {
    let owner = AgentId::new(1);
    let assignee = AgentId::new(2);
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
                .with(Operation::Delegate)
                .with(Operation::Verify),
        )
        .unwrap();
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
    let image = core
        .register_agent_image(
            owner,
            owner_capability,
            resource,
            AgentImageKind::Worker,
            AgentImageDigest::new([0x55; 32]),
            1,
            1,
        )
        .unwrap();
    core.verify_agent_image(owner, owner_capability, image)
        .unwrap();
    core.launch_task_agent(assignee, capability, task, image, AgentEntryKind::Worker)
        .unwrap();
    core.accept_task(assignee, task).unwrap();
    core.enqueue_task(assignee, task).unwrap();
    core.dispatch_next_with_quantum(assignee, 2).unwrap();
    RunningTask {
        assignee,
        task,
        capability,
    }
}

#[test]
fn running_assignee_submits_replayable_result_without_leaving_running_state() {
    let mut core = TestCore::<24>::new();
    let running = running_task(&mut core);
    let result = TaskResult {
        code: 0x0a01,
        value: 0xa11c_0001,
    };

    let event = core
        .submit_task_result(running.assignee, running.capability, running.task, result)
        .unwrap();

    assert_eq!(event.kind, EventKind::TaskResultSubmitted);
    assert_eq!(event.agent, running.assignee);
    assert_eq!(event.capability, Some(running.capability));
    assert_eq!(event.task, Some(running.task));
    assert_eq!(event.task_result, Some(result));
    let task = core
        .tasks()
        .iter()
        .find(|task| task.id == running.task)
        .unwrap();
    assert_eq!(task.result, Some(result));
    assert_eq!(task.status, TaskStatus::Running);
    assert_eq!(task.quantum_remaining, 2);
    let context = core.execution_context(running.assignee).unwrap();
    assert_eq!(context.state, AgentExecutionState::Running);
    assert_eq!(context.task, Some(running.task));
    assert!(core.run_queue().is_empty());

    core.complete_task(running.assignee, running.capability, running.task)
        .unwrap();
    assert_eq!(core.tasks()[0].result, Some(result));
}

#[test]
fn duplicate_result_is_rejected_without_replacing_value_or_recording_event() {
    let mut core = TestCore::<24>::new();
    let running = running_task(&mut core);
    let first = TaskResult { code: 1, value: 2 };
    core.submit_task_result(running.assignee, running.capability, running.task, first)
        .unwrap();
    let events_before = core.events().len();

    let error = core.submit_task_result(
        running.assignee,
        running.capability,
        running.task,
        TaskResult { code: 3, value: 4 },
    );

    assert_eq!(error, Err(KernelError::TaskResultAlreadySubmitted));
    assert_eq!(core.tasks()[0].result, Some(first));
    assert_eq!(core.events().len(), events_before);
}

#[test]
fn full_event_log_leaves_running_task_result_empty() {
    let mut core = TestCore::<14>::new();
    let running = running_task(&mut core);
    assert_eq!(core.events().len(), 14);

    let error = core.submit_task_result(
        running.assignee,
        running.capability,
        running.task,
        TaskResult { code: 5, value: 6 },
    );

    assert_eq!(error, Err(KernelError::EventLogFull));
    assert_eq!(core.tasks()[0].result, None);
    assert_eq!(core.tasks()[0].status, TaskStatus::Running);
    assert_eq!(core.events().len(), 14);
}
