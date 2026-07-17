//! Shared fixture for fault routes that target a blocked Handler Agent.

use agent_kernel_core::{
    AgentEntryKind, AgentExecutionState, AgentId, AgentImageDigest, AgentImageKind, CapabilityId,
    FaultId, FaultKind, FaultPolicyAction, IntentKind, KernelCore, MessageReceiveOutcome,
    MessageStatus, Operation, OperationSet, ResourceId, ResourceKind, TaskId, TaskStatus,
    VerificationRequirement,
};

pub(super) type WaitRouteCore = KernelCore<4, 1, 4, 96, 0, 0, 0, 3, 3, 1, 1, 0, 0, 1, 1, 1, 1>;

#[derive(Copy, Clone)]
pub(super) struct WaitingFaultRoute {
    pub(super) owner: AgentId,
    fault_worker: AgentId,
    pub(super) handler: AgentId,
    pub(super) spare: AgentId,
    pub(super) owner_capability: CapabilityId,
    pub(super) resource: ResourceId,
    pub(super) fault_task: TaskId,
    handler_task: TaskId,
    pub(super) spare_task: TaskId,
    pub(super) fault: FaultId,
}

pub(super) fn waiting_fault_route(core: &mut WaitRouteCore) -> WaitingFaultRoute {
    let owner = AgentId::new(1);
    let fault_worker = AgentId::new(2);
    let handler = AgentId::new(3);
    let spare = AgentId::new(4);
    for agent in [owner, fault_worker, handler, spare] {
        core.register_agent(agent).expect("agent should register");
    }
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
                .with(Operation::Verify)
                .with(Operation::Rollback),
        )
        .expect("owner capability should fit");

    let (handler_task, handler_capability) = delegated_task(
        core,
        owner,
        owner_capability,
        resource,
        handler,
        AgentImageKind::FaultHandler,
        AgentEntryKind::FaultHandler,
        1,
    );
    let (fault_task, _) = delegated_task(
        core,
        owner,
        owner_capability,
        resource,
        fault_worker,
        AgentImageKind::Worker,
        AgentEntryKind::Worker,
        2,
    );
    let (spare_task, _) = delegated_task(
        core,
        owner,
        owner_capability,
        resource,
        spare,
        AgentImageKind::Worker,
        AgentEntryKind::Worker,
        3,
    );
    core.install_fault_handler(
        owner,
        owner_capability,
        resource,
        FaultKind::ExecutionTrap,
        handler,
    )
    .expect("handler should install");
    core.install_fault_policy(
        owner,
        owner_capability,
        resource,
        FaultKind::ExecutionTrap,
        FaultPolicyAction::RouteToHandler,
    )
    .expect("route policy should install");

    core.enqueue_task(handler, handler_task)
        .expect("handler should queue");
    core.dispatch_next_with_quantum(handler, 1)
        .expect("handler should dispatch");
    assert!(matches!(
        core.receive_or_wait_message(handler, handler_capability, handler_task),
        Ok(MessageReceiveOutcome::Waiting(_))
    ));
    core.enqueue_task(fault_worker, fault_task)
        .expect("fault worker should queue");
    core.dispatch_next_with_quantum(fault_worker, 1)
        .expect("fault worker should dispatch");
    let fault = core
        .fault_task(fault_worker, fault_task, FaultKind::ExecutionTrap, 6)
        .expect("worker should fault");

    WaitingFaultRoute {
        owner,
        fault_worker,
        handler,
        spare,
        owner_capability,
        resource,
        fault_task,
        handler_task,
        spare_task,
        fault,
    }
}

#[allow(clippy::too_many_arguments)]
fn delegated_task(
    core: &mut WaitRouteCore,
    owner: AgentId,
    owner_capability: CapabilityId,
    resource: ResourceId,
    agent: AgentId,
    image_kind: AgentImageKind,
    entry_kind: AgentEntryKind,
    digest: u8,
) -> (TaskId, CapabilityId) {
    let intent = core
        .declare_intent(
            owner,
            owner_capability,
            resource,
            IntentKind::Act,
            VerificationRequirement::Required,
        )
        .expect("intent should declare");
    let task = core
        .create_task(owner, owner_capability, intent)
        .expect("task should create");
    let capability = core
        .delegate_task(owner, owner_capability, task, agent)
        .expect("task should delegate")
        .capability
        .expect("delegation should derive capability");
    let image = core
        .register_agent_image(
            owner,
            owner_capability,
            resource,
            image_kind,
            AgentImageDigest::new([digest; 32]),
            1,
            1,
        )
        .expect("image should register");
    core.verify_agent_image(owner, owner_capability, image)
        .expect("image should verify");
    core.launch_task_agent(agent, capability, task, image, entry_kind)
        .expect("task Agent should launch");
    core.accept_task(agent, task).expect("task should accept");
    (task, capability)
}

pub(super) fn assert_woken_handler(
    core: &WaitRouteCore,
    route: WaitingFaultRoute,
    message: agent_kernel_core::MessageId,
) {
    let record = core.messages()[0];
    assert_eq!(record.id, message);
    assert_eq!(record.sender, route.owner);
    assert_eq!(record.recipient, route.handler);
    assert_eq!(record.status, MessageStatus::Pending);
    assert_eq!(record.payload.resource, Some(route.resource));
    assert_eq!(record.payload.task, Some(route.fault_task));
    assert_eq!(record.payload.fault, Some(route.fault));
    assert_eq!(core.tasks()[0].id, route.handler_task);
    assert_eq!(core.tasks()[0].status, TaskStatus::Accepted);
    assert!(!core.waiters()[0].active);
    assert_eq!(core.run_queue().len(), 1);
    assert_eq!(core.run_queue()[0].agent, route.handler);
    assert_eq!(core.run_queue()[0].task, route.handler_task);
    let context = core
        .execution_contexts()
        .iter()
        .find(|context| context.agent == route.handler)
        .expect("handler context should exist");
    assert_eq!(context.state, AgentExecutionState::Idle);
    assert_eq!(context.task, None);
    assert_eq!(
        core.tasks()
            .iter()
            .find(|task| task.id == route.fault_task)
            .expect("fault task should exist")
            .status,
        TaskStatus::Faulted
    );
    assert_eq!(route.fault_worker, AgentId::new(2));
}
