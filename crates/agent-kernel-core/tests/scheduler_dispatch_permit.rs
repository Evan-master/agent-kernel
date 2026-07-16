use agent_kernel_core::{
    AgentEntryKind, AgentExecutionState, AgentId, AgentImageDigest, AgentImageKind, IntentKind,
    KernelCore, KernelError, Operation, OperationSet, ResourceKind, RunQueueEntry, TaskId,
    TaskStatus, VerificationRequirement,
};

type TestCore = KernelCore<2, 1, 2, 20, 0, 0, 0, 1, 1, 1>;

fn queued_task(core: &mut TestCore) -> (AgentId, TaskId) {
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
    core.delegate_task(owner, owner_capability, task, assignee)
        .unwrap();
    let task_capability = core.tasks()[0].delegated_capability.unwrap();
    let image = core
        .register_agent_image(
            owner,
            owner_capability,
            resource,
            AgentImageKind::Worker,
            AgentImageDigest::new([7; 32]),
            1,
            1,
        )
        .unwrap();
    core.verify_agent_image(owner, owner_capability, image)
        .unwrap();
    core.launch_task_agent(
        assignee,
        task_capability,
        task,
        image,
        AgentEntryKind::Worker,
    )
    .unwrap();
    core.accept_task(assignee, task).unwrap();
    core.enqueue_task(assignee, task).unwrap();
    (assignee, task)
}

#[test]
fn prepare_is_read_only_and_commit_dispatches_the_permit_entry() {
    let mut core = TestCore::new();
    let (agent, task) = queued_task(&mut core);
    let entry = RunQueueEntry { agent, task };
    let events_before = core.events().len();

    let permit = core
        .prepare_next_ready_dispatch_with_quantum(3)
        .expect("ready queue head should produce a permit");

    assert_eq!(permit.entry(), entry);
    assert_eq!(permit.quantum(), 3);
    assert_eq!(core.run_queue(), &[entry]);
    assert_eq!(core.tasks()[0].status, TaskStatus::Accepted);
    assert_eq!(
        core.execution_contexts()[1].state,
        AgentExecutionState::Idle
    );
    assert_eq!(core.events().len(), events_before);

    assert_eq!(core.commit_ready_dispatch(permit), Ok(entry));
    assert!(core.run_queue().is_empty());
    assert_eq!(core.tasks()[0].status, TaskStatus::Running);
    assert_eq!(core.tasks()[0].quantum_remaining, 3);
    assert_eq!(core.events().len(), events_before + 1);
}

#[test]
fn stale_permit_cannot_dispatch_after_the_queue_head_changes() {
    let mut core = TestCore::new();
    let (agent, task) = queued_task(&mut core);
    let permit = core
        .prepare_next_ready_dispatch_with_quantum(2)
        .expect("ready queue head should produce a permit");
    assert_eq!(
        core.dispatch_next_ready_with_quantum(2),
        Ok(RunQueueEntry { agent, task })
    );
    let events_before = core.events().len();

    assert_eq!(
        core.commit_ready_dispatch(permit),
        Err(KernelError::TaskNotRunnable)
    );
    assert!(core.run_queue().is_empty());
    assert_eq!(core.tasks()[0].status, TaskStatus::Running);
    assert_eq!(core.events().len(), events_before);
}

#[test]
fn prepare_rejects_invalid_quantum_and_empty_queue_without_mutation() {
    let empty = TestCore::new();
    assert_eq!(
        empty.prepare_next_ready_dispatch_with_quantum(1),
        Err(KernelError::RunQueueEmpty)
    );
    assert_eq!(
        empty.prepare_next_ready_dispatch_with_quantum(0),
        Err(KernelError::TaskQuantumInvalid)
    );
    assert!(empty.events().is_empty());
}
