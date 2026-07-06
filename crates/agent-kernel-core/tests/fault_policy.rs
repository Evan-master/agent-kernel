use agent_kernel_core::{
    AgentEntryKind, AgentId, CapabilityId, EventKind, FaultKind, FaultPolicyAction, FaultPolicyId,
    IntentKind, KernelCore, MessageId, MessageKind, Operation, OperationSet, ResourceId,
    ResourceKind, TaskId, TaskStatus, VerificationRequirement,
};

type InstallCore = KernelCore<1, 1, 1, 8, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1>;
type RouteCore = KernelCore<3, 1, 3, 32, 0, 0, 0, 1, 1, 1, 2, 0, 0, 1, 1, 1>;
type RecoverCore = KernelCore<2, 1, 2, 24, 0, 0, 0, 1, 1, 1, 0, 0, 0, 1, 0, 1>;

fn installable_core() -> (InstallCore, AgentId, CapabilityId, ResourceId) {
    let mut core = InstallCore::new();
    let owner = AgentId::new(1);
    core.register_agent(owner).expect("owner should register");
    let resource = core
        .register_resource(ResourceKind::Workspace, None)
        .expect("resource should fit");
    let capability = core
        .grant_capability(owner, resource, OperationSet::only(Operation::Rollback))
        .expect("rollback capability should fit");
    (core, owner, capability, resource)
}

fn route_running_task(core: &mut RouteCore) -> (AgentId, AgentId, CapabilityId, TaskId) {
    let owner = AgentId::new(2);
    let assignee = AgentId::new(3);
    let handler = AgentId::new(4);
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
        .expect("owner capability should fit");
    core.install_fault_handler(
        owner,
        capability,
        resource,
        FaultKind::ExecutionTrap,
        handler,
    )
    .expect("handler should install");
    core.install_fault_policy(
        owner,
        capability,
        resource,
        FaultKind::ExecutionTrap,
        FaultPolicyAction::RouteToHandler,
    )
    .expect("route policy should install");
    create_running_task(core, owner, assignee, capability, resource);
    (owner, assignee, capability, core.tasks()[0].id)
}

fn recover_running_task(core: &mut RecoverCore) -> (AgentId, AgentId, CapabilityId, TaskId) {
    let owner = AgentId::new(5);
    let assignee = AgentId::new(6);
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
        .expect("owner capability should fit");
    core.install_fault_policy(
        owner,
        capability,
        resource,
        FaultKind::ResourceFault,
        FaultPolicyAction::RecoverTask,
    )
    .expect("recover policy should install");
    create_running_task(core, owner, assignee, capability, resource);
    (owner, assignee, capability, core.tasks()[0].id)
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
    resource: ResourceId,
) {
    let intent = core
        .declare_intent(
            owner,
            capability,
            resource,
            IntentKind::Act,
            VerificationRequirement::Required,
        )
        .expect("intent should be declared");
    let task = core
        .create_task(owner, capability, intent)
        .expect("task should be created");
    let delegated_capability = core
        .delegate_task(owner, capability, task, assignee)
        .expect("task should delegate")
        .capability
        .expect("delegation should derive capability");
    core.launch_task_agent(assignee, delegated_capability, task, AgentEntryKind::Worker)
        .expect("assignee should launch for delegated task");
    core.accept_task(assignee, task)
        .expect("task should be accepted");
    core.enqueue_task(assignee, task)
        .expect("task should enqueue");
    core.dispatch_next_with_quantum(assignee, 2)
        .expect("task should dispatch");
}

#[test]
fn install_fault_policy_records_policy_and_event() {
    let (mut core, owner, capability, resource) = installable_core();

    let policy = core
        .install_fault_policy(
            owner,
            capability,
            resource,
            FaultKind::ExecutionTrap,
            FaultPolicyAction::RouteToHandler,
        )
        .expect("rollback authority should install policy");

    assert_eq!(policy, FaultPolicyId::new(1));
    assert_eq!(core.fault_policies().len(), 1);
    assert_eq!(core.fault_policies()[0].id, policy);
    assert_eq!(core.fault_policies()[0].resource, resource);
    assert_eq!(core.fault_policies()[0].kind, FaultKind::ExecutionTrap);
    assert_eq!(core.fault_policies()[0].installer, owner);
    assert_eq!(
        core.fault_policies()[0].action,
        FaultPolicyAction::RouteToHandler
    );
    let event = core.events().last().expect("install should record event");
    assert_eq!(event.kind, EventKind::FaultPolicyInstalled);
    assert_eq!(event.fault_policy, Some(policy));
    assert_eq!(
        event.fault_policy_action,
        Some(FaultPolicyAction::RouteToHandler)
    );
}

#[test]
fn apply_route_policy_routes_fault_and_records_policy_event() {
    let mut core = RouteCore::new();
    let (owner, assignee, capability, task) = route_running_task(&mut core);
    let fault = core
        .fault_task(assignee, task, FaultKind::ExecutionTrap, 7)
        .expect("task should fault");

    let outcome = core
        .apply_fault_policy(owner, capability, fault)
        .expect("policy should apply");

    assert_eq!(outcome.action, FaultPolicyAction::RouteToHandler);
    assert_eq!(outcome.message, Some(MessageId::new(1)));
    assert_eq!(outcome.event.kind, EventKind::FaultPolicyApplied);
    assert_eq!(core.messages()[0].kind, MessageKind::Fault);
    assert_eq!(core.messages()[0].recipient, AgentId::new(4));
    assert_eq!(core.messages()[0].payload.fault, Some(fault));
    assert_eq!(core.tasks()[0].status, TaskStatus::Faulted);
    let events = core.events();
    assert_eq!(events[events.len() - 3].kind, EventKind::MessageSent);
    assert_eq!(events[events.len() - 2].kind, EventKind::FaultRouted);
    assert_eq!(events[events.len() - 1].kind, EventKind::FaultPolicyApplied);
    assert_eq!(events[events.len() - 1].fault, Some(fault));
    assert_eq!(
        events[events.len() - 1].fault_policy_action,
        Some(FaultPolicyAction::RouteToHandler)
    );
}

#[test]
fn apply_recover_policy_recovers_task_and_records_policy_event() {
    let mut core = RecoverCore::new();
    let (owner, assignee, capability, task) = recover_running_task(&mut core);
    let fault = core
        .fault_task(assignee, task, FaultKind::ResourceFault, 9)
        .expect("task should fault");

    let outcome = core
        .apply_fault_policy(owner, capability, fault)
        .expect("recover policy should apply");

    assert_eq!(outcome.action, FaultPolicyAction::RecoverTask);
    assert_eq!(outcome.message, None);
    assert_eq!(outcome.event.kind, EventKind::FaultPolicyApplied);
    assert_eq!(core.tasks()[0].status, TaskStatus::Accepted);
    assert!(core.messages().is_empty());
    let events = core.events();
    assert_eq!(events[events.len() - 2].kind, EventKind::TaskFaultRecovered);
    assert_eq!(events[events.len() - 1].kind, EventKind::FaultPolicyApplied);
    assert_eq!(
        events[events.len() - 1].fault_policy_action,
        Some(FaultPolicyAction::RecoverTask)
    );
}
