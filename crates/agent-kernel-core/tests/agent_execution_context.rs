use agent_kernel_core::{
    AgentExecutionState, AgentId, EventKind, FaultKind, IntentKind, KernelCore, Operation,
    OperationSet, ResourceId, ResourceKind, SignalKey, TaskId, TaskStatus, VerificationRequirement,
};

type TestCore = KernelCore<3, 3, 8, 48, 0, 0, 0, 8, 8, 8, 0, 0, 0, 2, 0, 0, 2>;

struct PreparedTask {
    owner: AgentId,
    assignee: AgentId,
    resource: ResourceId,
    owner_capability: agent_kernel_core::CapabilityId,
    assignee_capability: agent_kernel_core::CapabilityId,
    task: TaskId,
}

fn accepted_task(core: &mut TestCore) -> PreparedTask {
    let owner = AgentId::new(1);
    let assignee = AgentId::new(2);
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
        .expect("intent should fit");
    let task = core
        .create_task(owner, owner_capability, intent)
        .expect("task should fit");
    core.delegate_task(owner, owner_capability, task, assignee)
        .expect("task should delegate");
    let assignee_capability = core.tasks()[0]
        .delegated_capability
        .expect("delegation should derive capability");
    core.accept_task(assignee, task)
        .expect("task should be accepted");

    PreparedTask {
        owner,
        assignee,
        resource,
        owner_capability,
        assignee_capability,
        task,
    }
}

#[test]
fn register_agent_creates_idle_execution_context() {
    let mut core = KernelCore::<1, 0, 0, 1, 0, 0, 0, 0, 0, 0>::new();
    let agent = AgentId::new(7);

    let event = core.register_agent(agent).expect("agent should register");
    let context = core
        .execution_context(agent)
        .expect("registered agent should have a context");

    assert_eq!(event.kind, EventKind::AgentRegistered);
    assert_eq!(core.execution_contexts().len(), 1);
    assert_eq!(context.agent, agent);
    assert_eq!(context.state, AgentExecutionState::Idle);
    assert_eq!(context.task, None);
    assert_eq!(context.run_ticks, 0);
    assert_eq!(context.quantum_remaining, 0);
}

#[test]
fn dispatch_tick_and_quantum_expiry_update_execution_context() {
    let mut core = TestCore::new();
    let prepared = accepted_task(&mut core);

    core.enqueue_task(prepared.assignee, prepared.task)
        .expect("task should enqueue");
    core.dispatch_next_with_quantum(prepared.assignee, 2)
        .expect("task should dispatch");

    let context = core
        .execution_context(prepared.assignee)
        .expect("assignee context should exist");
    assert_eq!(context.state, AgentExecutionState::Running);
    assert_eq!(context.task, Some(prepared.task));
    assert_eq!(context.run_ticks, 0);
    assert_eq!(context.quantum_remaining, 2);

    core.tick_task(prepared.assignee, prepared.task)
        .expect("first tick should remain running");
    let context = core
        .execution_context(prepared.assignee)
        .expect("assignee context should exist");
    assert_eq!(context.state, AgentExecutionState::Running);
    assert_eq!(context.task, Some(prepared.task));
    assert_eq!(context.run_ticks, 1);
    assert_eq!(context.quantum_remaining, 1);

    let event = core
        .tick_task(prepared.assignee, prepared.task)
        .expect("second tick should expire quantum");
    let context = core
        .execution_context(prepared.assignee)
        .expect("assignee context should exist");
    assert_eq!(event.kind, EventKind::TaskQuantumExpired);
    assert_eq!(context.state, AgentExecutionState::Idle);
    assert_eq!(context.task, None);
    assert_eq!(context.run_ticks, 0);
    assert_eq!(context.quantum_remaining, 0);
    assert_eq!(core.tasks()[0].status, TaskStatus::Accepted);
    assert_eq!(core.tasks()[0].run_ticks, 2);
}

#[test]
fn yield_wait_wake_fault_recover_and_complete_update_execution_context() {
    let mut core = TestCore::new();
    let prepared = accepted_task(&mut core);

    core.enqueue_task(prepared.assignee, prepared.task)
        .expect("task should enqueue");
    core.dispatch_next_with_quantum(prepared.assignee, 3)
        .expect("task should dispatch");
    core.yield_task(prepared.assignee, prepared.task)
        .expect("running task should yield");
    assert_eq!(
        core.execution_context(prepared.assignee).unwrap().state,
        AgentExecutionState::Idle
    );

    core.dispatch_next_with_quantum(prepared.assignee, 3)
        .expect("yielded task should dispatch");
    let signal = SignalKey::new(1);
    core.wait_task(
        prepared.assignee,
        prepared.assignee_capability,
        prepared.task,
        prepared.resource,
        signal,
    )
    .expect("running task should wait");
    let context = core.execution_context(prepared.assignee).unwrap();
    assert_eq!(context.state, AgentExecutionState::Waiting);
    assert_eq!(context.task, Some(prepared.task));

    core.emit_signal(
        prepared.owner,
        prepared.owner_capability,
        prepared.resource,
        signal,
    )
    .expect("signal should wake waiting task");
    assert_eq!(
        core.execution_context(prepared.assignee).unwrap().state,
        AgentExecutionState::Idle
    );

    core.dispatch_next_with_quantum(prepared.assignee, 2)
        .expect("woken task should dispatch");
    core.fault_task(
        prepared.assignee,
        prepared.task,
        FaultKind::ExecutionTrap,
        9,
    )
    .expect("running task should fault");
    let context = core.execution_context(prepared.assignee).unwrap();
    assert_eq!(context.state, AgentExecutionState::Faulted);
    assert_eq!(context.task, Some(prepared.task));

    core.recover_faulted_task(prepared.owner, prepared.owner_capability, prepared.task)
        .expect("owner should recover task");
    assert_eq!(
        core.execution_context(prepared.assignee).unwrap().state,
        AgentExecutionState::Idle
    );

    core.enqueue_task(prepared.assignee, prepared.task)
        .expect("recovered task should enqueue");
    core.dispatch_next(prepared.assignee)
        .expect("recovered task should dispatch");
    core.complete_task(
        prepared.assignee,
        prepared.assignee_capability,
        prepared.task,
    )
    .expect("running task should complete");
    assert_eq!(
        core.execution_context(prepared.assignee).unwrap().state,
        AgentExecutionState::Idle
    );
    assert_eq!(core.tasks()[0].status, TaskStatus::Completed);
}

#[test]
fn cancel_running_task_clears_assignee_execution_context() {
    let mut core = TestCore::new();
    let prepared = accepted_task(&mut core);

    core.enqueue_task(prepared.assignee, prepared.task)
        .expect("task should enqueue");
    core.dispatch_next(prepared.assignee)
        .expect("task should dispatch");
    core.cancel_task(prepared.owner, prepared.owner_capability, prepared.task)
        .expect("owner should cancel running task");

    let context = core.execution_context(prepared.assignee).unwrap();
    assert_eq!(context.state, AgentExecutionState::Idle);
    assert_eq!(context.task, None);
    assert_eq!(core.tasks()[0].status, TaskStatus::Cancelled);
}
