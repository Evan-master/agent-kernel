use agent_kernel_core::{
    AgentId, CapabilityId, EventKind, FaultHandlerId, FaultKind, IntentKind, KernelCore, MessageId,
    MessageKind, MessageStatus, Operation, OperationSet, ResourceId, ResourceKind, TaskId,
    VerificationRequirement,
};

type HandlerCore = KernelCore<2, 1, 1, 8, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1>;
type RouteCore = KernelCore<3, 1, 3, 24, 0, 0, 0, 1, 1, 1, 2, 0, 0, 1, 1>;

#[derive(Copy, Clone)]
struct RunningFault {
    owner: AgentId,
    assignee: AgentId,
    handler: AgentId,
    owner_capability: CapabilityId,
    resource: ResourceId,
    task: TaskId,
}

fn installable_core() -> (HandlerCore, AgentId, AgentId, CapabilityId, ResourceId) {
    let mut core = HandlerCore::new();
    let owner = AgentId::new(1);
    let handler = AgentId::new(2);
    core.register_agent(owner).expect("owner should register");
    core.register_agent(handler)
        .expect("handler should register");
    let resource = core
        .register_resource(ResourceKind::Workspace, None)
        .expect("resource should fit");
    let capability = core
        .grant_capability(owner, resource, OperationSet::only(Operation::Rollback))
        .expect("rollback capability should fit");
    (core, owner, handler, capability, resource)
}

fn running_fault(core: &mut RouteCore) -> RunningFault {
    let owner = AgentId::new(3);
    let assignee = AgentId::new(4);
    let handler = AgentId::new(5);
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
    core.install_fault_handler(
        owner,
        owner_capability,
        resource,
        FaultKind::ExecutionTrap,
        handler,
    )
    .expect("handler should install");
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

    RunningFault {
        owner,
        assignee,
        handler,
        owner_capability,
        resource,
        task,
    }
}

#[test]
fn install_fault_handler_records_handler_and_event() {
    let (mut core, owner, handler, capability, resource) = installable_core();

    let handler_id = core
        .install_fault_handler(
            owner,
            capability,
            resource,
            FaultKind::ExecutionTrap,
            handler,
        )
        .expect("rollback authority should install handler");

    assert_eq!(handler_id, FaultHandlerId::new(1));
    assert_eq!(core.fault_handlers().len(), 1);
    assert_eq!(core.fault_handlers()[0].id, handler_id);
    assert_eq!(core.fault_handlers()[0].resource, resource);
    assert_eq!(core.fault_handlers()[0].kind, FaultKind::ExecutionTrap);
    assert_eq!(core.fault_handlers()[0].installer, owner);
    assert_eq!(core.fault_handlers()[0].handler, handler);
    let event = core.events().last().expect("install should record event");
    assert_eq!(event.kind, EventKind::FaultHandlerInstalled);
    assert_eq!(event.resource, Some(resource));
    assert_eq!(event.capability, Some(capability));
    assert_eq!(event.fault_kind, Some(FaultKind::ExecutionTrap));
    assert_eq!(event.target_agent, Some(handler));
}

#[test]
fn route_fault_to_handler_sends_fault_message_and_event() {
    let mut core = RouteCore::new();
    let prepared = running_fault(&mut core);
    let fault = core
        .fault_task(
            prepared.assignee,
            prepared.task,
            FaultKind::ExecutionTrap,
            7,
        )
        .expect("task should fault");

    let message = core
        .route_fault_to_handler(prepared.owner, prepared.owner_capability, fault)
        .expect("fault should route to installed handler");

    assert_eq!(message, MessageId::new(1));
    assert_eq!(core.messages().len(), 1);
    let record = core.messages()[0];
    assert_eq!(record.sender, prepared.owner);
    assert_eq!(record.recipient, prepared.handler);
    assert_eq!(record.kind, MessageKind::Fault);
    assert_eq!(record.status, MessageStatus::Pending);
    assert_eq!(record.payload.resource, Some(prepared.resource));
    assert_eq!(record.payload.task, Some(prepared.task));
    assert_eq!(record.payload.fault, Some(fault));

    let events = core.events();
    let sent = events[events.len() - 2];
    let routed = events[events.len() - 1];
    assert_eq!(sent.kind, EventKind::MessageSent);
    assert_eq!(sent.message, Some(message));
    assert_eq!(routed.kind, EventKind::FaultRouted);
    assert_eq!(routed.message, Some(message));
    assert_eq!(routed.task, Some(prepared.task));
    assert_eq!(routed.fault, Some(fault));
    assert_eq!(routed.fault_kind, Some(FaultKind::ExecutionTrap));
    assert_eq!(routed.fault_detail, Some(7));
    assert_eq!(routed.target_agent, Some(prepared.handler));
}
