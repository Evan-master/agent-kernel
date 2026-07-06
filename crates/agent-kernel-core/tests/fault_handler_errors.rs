use agent_kernel_core::{
    AgentId, CapabilityId, EventKind, FaultKind, IntentKind, KernelCore, KernelError, Operation,
    OperationSet, ResourceKind, TaskId, TaskStatus, VerificationRequirement,
};

#[derive(Copy, Clone)]
struct PreparedFault {
    owner: AgentId,
    assignee: AgentId,
    owner_capability: CapabilityId,
    task: TaskId,
}

fn prepare_fault<const EVENTS: usize, const MESSAGES: usize, const FAULT_HANDLERS: usize>(
    core: &mut KernelCore<3, 1, 3, EVENTS, 0, 0, 0, 1, 1, 1, MESSAGES, 0, 0, 1, FAULT_HANDLERS>,
    install_handler: bool,
) -> PreparedFault {
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
    if install_handler {
        core.install_fault_handler(
            owner,
            owner_capability,
            resource,
            FaultKind::ExecutionTrap,
            handler,
        )
        .expect("handler should install");
    }
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
    core.accept_task(assignee, task)
        .expect("task should be accepted");
    core.enqueue_task(assignee, task)
        .expect("task should enqueue");
    core.dispatch_next_with_quantum(assignee, 2)
        .expect("task should dispatch");

    PreparedFault {
        owner,
        assignee,
        owner_capability,
        task,
    }
}

#[test]
fn install_fault_handler_requires_rollback_authority_without_mutation() {
    let mut core = KernelCore::<2, 1, 1, 8, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1>::new();
    let owner = AgentId::new(4);
    let handler = AgentId::new(5);
    core.register_agent(owner).expect("owner should register");
    core.register_agent(handler)
        .expect("handler should register");
    let resource = core
        .register_resource(ResourceKind::Workspace, None)
        .expect("resource should fit");
    let capability = core
        .grant_capability(owner, resource, OperationSet::only(Operation::Act))
        .expect("act capability should fit");
    let events_before = core.events().len();

    assert_eq!(
        core.install_fault_handler(
            owner,
            capability,
            resource,
            FaultKind::ExecutionTrap,
            handler,
        ),
        Err(KernelError::OperationDenied)
    );
    assert!(core.fault_handlers().is_empty());
    assert_eq!(core.events().len(), events_before);
}

#[test]
fn install_fault_handler_rejects_duplicate_without_mutation() {
    let mut core = KernelCore::<2, 1, 1, 8, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 2>::new();
    let owner = AgentId::new(6);
    let handler = AgentId::new(7);
    core.register_agent(owner).expect("owner should register");
    core.register_agent(handler)
        .expect("handler should register");
    let resource = core
        .register_resource(ResourceKind::Workspace, None)
        .expect("resource should fit");
    let capability = core
        .grant_capability(owner, resource, OperationSet::only(Operation::Rollback))
        .expect("rollback capability should fit");
    core.install_fault_handler(
        owner,
        capability,
        resource,
        FaultKind::ExecutionTrap,
        handler,
    )
    .expect("first install should succeed");
    let events_before = core.events().len();

    assert_eq!(
        core.install_fault_handler(
            owner,
            capability,
            resource,
            FaultKind::ExecutionTrap,
            handler,
        ),
        Err(KernelError::FaultHandlerAlreadyExists)
    );
    assert_eq!(core.fault_handlers().len(), 1);
    assert_eq!(core.events().len(), events_before);
}

#[test]
fn route_fault_requires_installed_handler_without_message_or_event() {
    let mut core = KernelCore::<3, 1, 3, 20, 0, 0, 0, 1, 1, 1, 1, 0, 0, 1, 0>::new();
    let prepared = prepare_fault(&mut core, false);
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
        core.route_fault_to_handler(prepared.owner, prepared.owner_capability, fault),
        Err(KernelError::FaultHandlerNotFound)
    );
    assert!(core.messages().is_empty());
    assert_eq!(core.events().len(), events_before);
}

#[test]
fn route_fault_rejects_recovered_fault_without_message_or_event() {
    let mut core = KernelCore::<3, 1, 3, 24, 0, 0, 0, 1, 1, 1, 1, 0, 0, 1, 1>::new();
    let prepared = prepare_fault(&mut core, true);
    let fault = core
        .fault_task(
            prepared.assignee,
            prepared.task,
            FaultKind::ExecutionTrap,
            2,
        )
        .expect("task should fault");
    core.recover_faulted_task(prepared.owner, prepared.owner_capability, prepared.task)
        .expect("fault should recover");
    let events_before = core.events().len();

    assert_eq!(
        core.route_fault_to_handler(prepared.owner, prepared.owner_capability, fault),
        Err(KernelError::TaskStatusMismatch)
    );
    assert!(core.messages().is_empty());
    assert_eq!(core.tasks()[0].status, TaskStatus::Accepted);
    assert_eq!(core.events().len(), events_before);
}

#[test]
fn route_fault_message_store_full_leaves_state_unchanged() {
    let mut core = KernelCore::<3, 1, 3, 24, 0, 0, 0, 1, 1, 1, 0, 0, 0, 1, 1>::new();
    let prepared = prepare_fault(&mut core, true);
    let fault = core
        .fault_task(
            prepared.assignee,
            prepared.task,
            FaultKind::ExecutionTrap,
            3,
        )
        .expect("task should fault");
    let events_before = core.events().len();

    assert_eq!(
        core.route_fault_to_handler(prepared.owner, prepared.owner_capability, fault),
        Err(KernelError::MessageStoreFull)
    );
    assert!(core.messages().is_empty());
    assert_eq!(core.tasks()[0].status, TaskStatus::Faulted);
    assert_eq!(core.events().len(), events_before);
}

#[test]
fn route_fault_event_log_full_leaves_state_unchanged() {
    let mut core = KernelCore::<3, 1, 3, 15, 0, 0, 0, 1, 1, 1, 1, 0, 0, 1, 1>::new();
    let prepared = prepare_fault(&mut core, true);
    let fault = core
        .fault_task(
            prepared.assignee,
            prepared.task,
            FaultKind::ExecutionTrap,
            4,
        )
        .expect("task should fault");

    assert_eq!(
        core.route_fault_to_handler(prepared.owner, prepared.owner_capability, fault),
        Err(KernelError::EventLogFull)
    );
    assert!(core.messages().is_empty());
    assert_eq!(core.tasks()[0].status, TaskStatus::Faulted);
    assert_eq!(core.events().len(), 14);
    assert_eq!(core.events().last().unwrap().kind, EventKind::TaskFaulted);
}
