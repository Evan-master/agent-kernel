use agent_kernel_core::{
    AgentEntryKind, AgentId, CapabilityId, EventKind, FaultId, FaultKind, IntentKind, KernelCore,
    Operation, OperationSet, ResourceKind, TaskId, TaskStatus, VerificationRequirement,
};

type TestCore = KernelCore<2, 1, 2, 24, 0, 0, 0, 1, 1, 1, 0, 0, 0, 2>;

#[derive(Copy, Clone)]
struct RunningTask {
    task: TaskId,
    owner_capability: CapabilityId,
}

fn running_task(core: &mut TestCore, owner: AgentId, assignee: AgentId) -> RunningTask {
    core.register_agent(owner).expect("owner should register");
    core.register_agent(assignee)
        .expect("assignee should register");
    let resource = core
        .register_resource(ResourceKind::Workspace, None)
        .expect("resource should fit");
    let owner_capability = core
        .grant_capability(
            owner,
            resource,
            OperationSet::empty()
                .with(Operation::Act)
                .with(Operation::Delegate)
                .with(Operation::Rollback),
        )
        .expect("owner capability should fit");
    let intent = core
        .declare_intent(
            owner,
            owner_capability,
            resource,
            IntentKind::Act,
            VerificationRequirement::Required,
        )
        .expect("intent should be declared");
    let task = core
        .create_task(owner, owner_capability, intent)
        .expect("task should be created");
    core.delegate_task(owner, owner_capability, task, assignee)
        .expect("task should be delegated");
    let assignee_capability = core.tasks()[0]
        .delegated_capability
        .expect("delegation should derive assignee capability");
    core.launch_task_agent(assignee, assignee_capability, task, AgentEntryKind::Worker)
        .expect("assignee should launch for delegated task");
    core.accept_task(assignee, task)
        .expect("task should be accepted");
    core.enqueue_task(assignee, task)
        .expect("task should enqueue");
    core.dispatch_next_with_quantum(assignee, 2)
        .expect("task should dispatch");

    RunningTask {
        task,
        owner_capability,
    }
}

#[test]
fn fault_running_task_records_fault_and_event() {
    let mut core = TestCore::new();
    let owner = AgentId::new(1);
    let assignee = AgentId::new(2);
    let running = running_task(&mut core, owner, assignee);

    let fault = core
        .fault_task(assignee, running.task, FaultKind::ExecutionTrap, 7)
        .expect("running task should fault");

    assert_eq!(fault, FaultId::new(1));
    assert_eq!(core.faults().len(), 1);
    assert_eq!(core.faults()[0].id, fault);
    assert_eq!(core.faults()[0].task, running.task);
    assert_eq!(core.faults()[0].agent, assignee);
    assert_eq!(core.faults()[0].kind, FaultKind::ExecutionTrap);
    assert_eq!(core.faults()[0].detail, 7);
    assert_eq!(core.tasks()[0].status, TaskStatus::Faulted);
    assert_eq!(core.tasks()[0].last_fault, Some(fault));
    assert_eq!(core.tasks()[0].quantum_remaining, 0);
    let event = core.events().last().expect("fault should record event");
    assert_eq!(event.kind, EventKind::TaskFaulted);
    assert_eq!(event.task, Some(running.task));
    assert_eq!(event.fault, Some(fault));
    assert_eq!(event.fault_kind, Some(FaultKind::ExecutionTrap));
    assert_eq!(event.fault_detail, Some(7));
}

#[test]
fn recover_faulted_task_records_recovery_event() {
    let mut core = TestCore::new();
    let owner = AgentId::new(3);
    let assignee = AgentId::new(4);
    let running = running_task(&mut core, owner, assignee);
    let fault = core
        .fault_task(assignee, running.task, FaultKind::ResourceFault, 9)
        .expect("running task should fault");

    let event = core
        .recover_faulted_task(owner, running.owner_capability, running.task)
        .expect("rollback authority should recover faulted task");

    assert_eq!(event.kind, EventKind::TaskFaultRecovered);
    assert_eq!(event.task, Some(running.task));
    assert_eq!(event.fault, Some(fault));
    assert_eq!(core.tasks()[0].status, TaskStatus::Accepted);
    assert_eq!(core.tasks()[0].last_fault, Some(fault));
    assert!(core.run_queue().is_empty());
}
