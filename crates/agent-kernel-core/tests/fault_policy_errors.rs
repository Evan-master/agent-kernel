use agent_kernel_core::{
    AgentId, CapabilityId, EventKind, FaultKind, FaultPolicyAction, IntentKind, KernelCore,
    KernelError, Operation, OperationSet, ResourceKind, TaskId, TaskStatus,
    VerificationRequirement,
};

#[derive(Copy, Clone)]
struct PreparedTask {
    owner: AgentId,
    assignee: AgentId,
    capability: CapabilityId,
    task: TaskId,
}

fn prepare_route_fault<const EVENTS: usize, const MESSAGES: usize, const FAULT_POLICIES: usize>(
    core: &mut KernelCore<3, 1, 3, EVENTS, 0, 0, 0, 1, 1, 1, MESSAGES, 0, 0, 1, 1, FAULT_POLICIES>,
    install_policy: bool,
) -> PreparedTask {
    let owner = AgentId::new(1);
    let assignee = AgentId::new(2);
    let handler = AgentId::new(3);
    core.register_agent(owner).expect("owner should register");
    core.register_agent(assignee)
        .expect("assignee should register");
    core.register_agent(handler)
        .expect("handler should register");
    let resource = core
        .register_resource(ResourceKind::Workspace, None)
        .expect("resource should fit");
    let capability = core
        .grant_capability(
            owner,
            resource,
            OperationSet::empty()
                .with(Operation::Act)
                .with(Operation::Delegate)
                .with(Operation::Rollback),
        )
        .expect("capability should fit");
    core.install_fault_handler(
        owner,
        capability,
        resource,
        FaultKind::ExecutionTrap,
        handler,
    )
    .expect("handler should install");
    if install_policy {
        core.install_fault_policy(
            owner,
            capability,
            resource,
            FaultKind::ExecutionTrap,
            FaultPolicyAction::RouteToHandler,
        )
        .expect("route policy should install");
    }
    let task = create_running_task(core, owner, assignee, capability, resource);
    PreparedTask {
        owner,
        assignee,
        capability,
        task,
    }
}

fn prepare_recover_fault<const EVENTS: usize>(
    core: &mut KernelCore<2, 1, 2, EVENTS, 0, 0, 0, 1, 1, 1, 0, 0, 0, 1, 0, 1>,
) -> PreparedTask {
    let owner = AgentId::new(4);
    let assignee = AgentId::new(5);
    core.register_agent(owner).expect("owner should register");
    core.register_agent(assignee)
        .expect("assignee should register");
    let resource = core
        .register_resource(ResourceKind::Workspace, None)
        .expect("resource should fit");
    let capability = core
        .grant_capability(
            owner,
            resource,
            OperationSet::empty()
                .with(Operation::Act)
                .with(Operation::Delegate)
                .with(Operation::Rollback),
        )
        .expect("capability should fit");
    core.install_fault_policy(
        owner,
        capability,
        resource,
        FaultKind::ResourceFault,
        FaultPolicyAction::RecoverTask,
    )
    .expect("recover policy should install");
    let task = create_running_task(core, owner, assignee, capability, resource);
    PreparedTask {
        owner,
        assignee,
        capability,
        task,
    }
}

fn create_running_task<
    const AGENTS: usize,
    const CAPS: usize,
    const EVENTS: usize,
    const MESSAGES: usize,
    const FAULT_HANDLERS: usize,
    const FAULT_POLICIES: usize,
>(
    core: &mut KernelCore<
        AGENTS,
        1,
        CAPS,
        EVENTS,
        0,
        0,
        0,
        1,
        1,
        1,
        MESSAGES,
        0,
        0,
        1,
        FAULT_HANDLERS,
        FAULT_POLICIES,
    >,
    owner: AgentId,
    assignee: AgentId,
    capability: CapabilityId,
    resource: agent_kernel_core::ResourceId,
) -> TaskId {
    let intent = core
        .declare_intent(
            owner,
            capability,
            resource,
            IntentKind::Act,
            VerificationRequirement::Required,
        )
        .expect("intent should declare");
    let task = core
        .create_task(owner, capability, intent)
        .expect("task should create");
    core.delegate_task(owner, capability, task, assignee)
        .expect("task should delegate");
    core.accept_task(assignee, task)
        .expect("task should accept");
    core.enqueue_task(assignee, task)
        .expect("task should enqueue");
    core.dispatch_next_with_quantum(assignee, 2)
        .expect("task should dispatch");
    task
}

#[test]
fn apply_fault_policy_requires_policy_without_message_or_event() {
    let mut core = KernelCore::<3, 1, 3, 24, 0, 0, 0, 1, 1, 1, 1, 0, 0, 1, 1, 0>::new();
    let prepared = prepare_route_fault(&mut core, false);
    let fault = core
        .fault_task(
            prepared.assignee,
            prepared.task,
            FaultKind::ExecutionTrap,
            1,
        )
        .expect("task should fault");
    let events_before = core.events().len();

    assert_eq!(
        core.apply_fault_policy(prepared.owner, prepared.capability, fault),
        Err(KernelError::FaultPolicyNotFound)
    );
    assert!(core.messages().is_empty());
    assert_eq!(core.events().len(), events_before);
}

#[test]
fn apply_route_policy_message_store_full_leaves_state_unchanged() {
    let mut core = KernelCore::<3, 1, 3, 24, 0, 0, 0, 1, 1, 1, 0, 0, 0, 1, 1, 1>::new();
    let prepared = prepare_route_fault(&mut core, true);
    let fault = core
        .fault_task(
            prepared.assignee,
            prepared.task,
            FaultKind::ExecutionTrap,
            2,
        )
        .expect("task should fault");
    let events_before = core.events().len();

    assert_eq!(
        core.apply_fault_policy(prepared.owner, prepared.capability, fault),
        Err(KernelError::MessageStoreFull)
    );
    assert!(core.messages().is_empty());
    assert_eq!(core.tasks()[0].status, TaskStatus::Faulted);
    assert_eq!(core.events().len(), events_before);
}

#[test]
fn apply_route_policy_event_log_full_leaves_state_unchanged() {
    let mut core = KernelCore::<3, 1, 3, 17, 0, 0, 0, 1, 1, 1, 1, 0, 0, 1, 1, 1>::new();
    let prepared = prepare_route_fault(&mut core, true);
    let fault = core
        .fault_task(
            prepared.assignee,
            prepared.task,
            FaultKind::ExecutionTrap,
            3,
        )
        .expect("task should fault");

    assert_eq!(
        core.apply_fault_policy(prepared.owner, prepared.capability, fault),
        Err(KernelError::EventLogFull)
    );
    assert!(core.messages().is_empty());
    assert_eq!(core.tasks()[0].status, TaskStatus::Faulted);
    assert_eq!(core.events().len(), 15);
    assert_eq!(core.events().last().unwrap().kind, EventKind::TaskFaulted);
}

#[test]
fn apply_recover_policy_event_log_full_leaves_task_faulted() {
    let mut core = KernelCore::<2, 1, 2, 14, 0, 0, 0, 1, 1, 1, 0, 0, 0, 1, 0, 1>::new();
    let prepared = prepare_recover_fault(&mut core);
    let fault = core
        .fault_task(
            prepared.assignee,
            prepared.task,
            FaultKind::ResourceFault,
            4,
        )
        .expect("task should fault");

    assert_eq!(
        core.apply_fault_policy(prepared.owner, prepared.capability, fault),
        Err(KernelError::EventLogFull)
    );
    assert_eq!(core.tasks()[0].status, TaskStatus::Faulted);
    assert_eq!(core.events().len(), 13);
    assert_eq!(core.events().last().unwrap().kind, EventKind::TaskFaulted);
}
