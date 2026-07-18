use agent_kernel_core::{
    AgentExecutionState, AgentId, AgentStatus, Event, EventKind, IntentKind, KernelCore,
    KernelError, Operation, OperationSet, ResourceId, ResourceKind, VerificationRequirement,
};

type TestCore = KernelCore<4, 1, 6, 32, 0, 0, 0, 2, 2, 1>;

fn setup(
    operations: OperationSet,
) -> (
    TestCore,
    AgentId,
    ResourceId,
    agent_kernel_core::CapabilityId,
) {
    let mut core = TestCore::new();
    let manager = AgentId::new(1);
    core.register_agent(manager)
        .expect("manager should register");
    let resource = core
        .register_resource(ResourceKind::Workspace, None)
        .expect("management resource should fit");
    let capability = core
        .grant_capability(manager, resource, operations)
        .expect("management capability should fit");
    (core, manager, resource, capability)
}

fn assert_management_event(
    event: Event,
    kind: EventKind,
    manager: AgentId,
    target: AgentId,
    resource: ResourceId,
    capability: agent_kernel_core::CapabilityId,
) {
    assert_eq!(event.kind, kind);
    assert_eq!(event.agent, manager);
    assert_eq!(event.target_agent, Some(target));
    assert_eq!(event.resource, Some(resource));
    assert_eq!(event.capability, Some(capability));
    assert_eq!(event.operation, Some(Operation::Delegate));
}

#[test]
fn managed_agent_lifecycle_records_authority_and_status() {
    let (mut core, manager, resource, capability) = setup(OperationSet::only(Operation::Delegate));
    let target = AgentId::new(9);

    let registered = core
        .register_managed_agent(manager, capability, resource, target)
        .expect("managed agent should register");

    assert_eq!(core.agents().len(), 2);
    assert_eq!(core.agents()[1].manager, Some(manager));
    assert_eq!(core.agents()[1].management_resource, Some(resource));
    assert_eq!(core.agents()[1].status, AgentStatus::Active);
    let context = core
        .execution_context(target)
        .expect("context should exist");
    assert_eq!(context.state, AgentExecutionState::Idle);
    assert_eq!(context.task, None);
    assert_management_event(
        registered,
        EventKind::AgentRegistered,
        manager,
        target,
        resource,
        capability,
    );

    let suspended = core
        .suspend_managed_agent(manager, capability, target)
        .expect("managed agent should suspend");
    let resumed = core
        .resume_managed_agent(manager, capability, target)
        .expect("managed agent should resume");
    let retired = core
        .retire_managed_agent(manager, capability, target)
        .expect("managed agent should retire");

    assert_eq!(core.agents()[1].status, AgentStatus::Retired);
    assert_management_event(
        suspended,
        EventKind::AgentSuspended,
        manager,
        target,
        resource,
        capability,
    );
    assert_management_event(
        resumed,
        EventKind::AgentResumed,
        manager,
        target,
        resource,
        capability,
    );
    assert_management_event(
        retired,
        EventKind::AgentRetired,
        manager,
        target,
        resource,
        capability,
    );
}

#[test]
fn managed_registration_requires_root_delegate_authority() {
    let (mut core, manager, resource, capability) = setup(OperationSet::only(Operation::Act));
    let events_before = core.events().len();

    let result = core.register_managed_agent(manager, capability, resource, AgentId::new(9));

    assert_eq!(result, Err(KernelError::OperationDenied));
    assert_eq!(core.agents().len(), 1);
    assert_eq!(core.events().len(), events_before);
}

#[test]
fn managed_lifecycle_rejects_self_and_trusted_targets() {
    let (mut core, manager, resource, capability) = setup(OperationSet::only(Operation::Delegate));
    assert_eq!(
        core.register_managed_agent(manager, capability, resource, manager),
        Err(KernelError::AgentSelfManagementDenied)
    );

    let trusted = AgentId::new(2);
    core.register_agent(trusted)
        .expect("trusted agent should register");
    let events_before = core.events().len();

    assert_eq!(
        core.suspend_managed_agent(manager, capability, trusted),
        Err(KernelError::AgentManagementDenied)
    );
    assert_eq!(core.agents()[1].status, AgentStatus::Active);
    assert_eq!(core.events().len(), events_before);
}

#[test]
fn delegated_manager_can_control_agent_on_same_resource() {
    let (mut core, creator, resource, capability) = setup(OperationSet::only(Operation::Delegate));
    let delegate = AgentId::new(2);
    core.register_agent(delegate)
        .expect("delegate should register");
    let delegated = core
        .derive_capability(
            creator,
            capability,
            delegate,
            OperationSet::only(Operation::Delegate),
        )
        .expect("delegate authority should derive");
    let target = AgentId::new(9);
    core.register_managed_agent(creator, capability, resource, target)
        .expect("creator should register target");

    let event = core
        .suspend_managed_agent(delegate, delegated, target)
        .expect("delegated manager should suspend target");

    assert_management_event(
        event,
        EventKind::AgentSuspended,
        delegate,
        target,
        resource,
        delegated,
    );
    assert_eq!(core.agents()[2].manager, Some(creator));
}

#[test]
fn active_assigned_task_blocks_managed_lifecycle_without_mutation() {
    let operations = OperationSet::empty()
        .with(Operation::Act)
        .with(Operation::Delegate);
    let (mut core, manager, resource, capability) = setup(operations);
    let target = AgentId::new(9);
    core.register_managed_agent(manager, capability, resource, target)
        .expect("managed agent should register");
    let intent = core
        .declare_intent(
            manager,
            capability,
            resource,
            IntentKind::Act,
            VerificationRequirement::Optional,
        )
        .expect("intent should be declared");
    let task = core
        .create_task(manager, capability, intent)
        .expect("task should be created");
    core.delegate_task(manager, capability, task, target)
        .expect("task should be delegated");
    let events_before = core.events().len();

    let result = core.suspend_managed_agent(manager, capability, target);

    assert_eq!(result, Err(KernelError::AgentManagementBusy));
    assert_eq!(core.agents()[1].status, AgentStatus::Active);
    assert_eq!(core.events().len(), events_before);
}

#[test]
fn event_capacity_failures_leave_managed_state_unchanged() {
    let mut core = KernelCore::<2, 1, 1, 3, 0, 0, 0, 0, 0, 0>::new();
    let manager = AgentId::new(1);
    core.register_agent(manager)
        .expect("manager should register");
    let resource = core
        .register_resource(ResourceKind::Workspace, None)
        .expect("resource should fit");
    let capability = core
        .grant_capability(manager, resource, OperationSet::only(Operation::Delegate))
        .expect("capability should fit");
    let target = AgentId::new(9);
    core.register_managed_agent(manager, capability, resource, target)
        .expect("managed registration should consume final event");

    assert_eq!(
        core.suspend_managed_agent(manager, capability, target),
        Err(KernelError::EventLogFull)
    );
    assert_eq!(core.agents()[1].status, AgentStatus::Active);
    assert_eq!(core.events().len(), 3);
}

#[test]
fn managed_registration_event_failure_does_not_insert_target() {
    let mut core = KernelCore::<2, 1, 1, 2, 0, 0, 0, 0, 0, 0>::new();
    let manager = AgentId::new(1);
    core.register_agent(manager)
        .expect("manager should register");
    let resource = core
        .register_resource(ResourceKind::Workspace, None)
        .expect("resource should fit");
    let capability = core
        .grant_capability(manager, resource, OperationSet::only(Operation::Delegate))
        .expect("capability should consume final event");

    assert_eq!(
        core.register_managed_agent(manager, capability, resource, AgentId::new(9)),
        Err(KernelError::EventLogFull)
    );
    assert_eq!(core.agents().len(), 1);
    assert_eq!(core.events().len(), 2);
}
