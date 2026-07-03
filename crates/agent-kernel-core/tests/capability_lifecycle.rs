use agent_kernel_core::{
    AgentId, CapabilityId, EventKind, IntentId, IntentKind, KernelCore, KernelError, Operation,
    OperationSet, ResourceId, ResourceKind, TaskStatus, VerificationRequirement,
};

type TestCore = KernelCore<4, 8, 32, 6, 6, 4>;

fn declare_action_intent(
    core: &mut TestCore,
    agent: AgentId,
    capability: CapabilityId,
    resource: ResourceId,
) -> IntentId {
    core.declare_intent(
        agent,
        capability,
        resource,
        IntentKind::Act,
        VerificationRequirement::Required,
    )
    .expect("intent should be declared")
}

#[test]
fn grant_capability_records_capability_granted_event() {
    let mut core = TestCore::new();
    let agent = AgentId::new(1);
    let resource = core
        .register_resource(ResourceKind::Workspace, None)
        .expect("resource should fit");
    let operations = OperationSet::empty()
        .with(Operation::Observe)
        .with(Operation::Act);

    let capability = core
        .grant_capability(agent, resource, operations)
        .expect("grant should fit");

    let events = core.events();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].kind, EventKind::CapabilityGranted);
    assert_eq!(events[0].agent, agent);
    assert_eq!(events[0].resource, Some(resource));
    assert_eq!(events[0].capability, Some(capability));
    assert_eq!(events[0].source_capability, None);
    assert_eq!(events[0].operations, operations);
    assert_eq!(events[0].task, None);
    assert_eq!(events[0].target_agent, None);
}

#[test]
fn grant_capability_returns_event_log_full_without_allocating() {
    let mut core = KernelCore::<1, 1, 0, 0, 0, 0>::new();
    let agent = AgentId::new(2);
    let resource = core
        .register_resource(ResourceKind::Workspace, None)
        .expect("resource should fit");

    let result = core.grant_capability(agent, resource, OperationSet::only(Operation::Observe));

    assert_eq!(result, Err(KernelError::EventLogFull));
    assert!(core.events().is_empty());
    assert_eq!(
        core.authorize(agent, CapabilityId::new(1), resource, Operation::Observe),
        Err(KernelError::CapabilityNotFound)
    );
}

#[test]
fn revoke_capability_records_capability_revoked_event() {
    let mut core = TestCore::new();
    let agent = AgentId::new(3);
    let resource = core
        .register_resource(ResourceKind::Service, None)
        .expect("resource should fit");
    let operations = OperationSet::only(Operation::Observe);
    let capability = core
        .grant_capability(agent, resource, operations)
        .expect("grant should fit");

    core.revoke_capability(capability)
        .expect("capability should revoke");

    let events = core.events();
    assert_eq!(events.len(), 2);
    assert_eq!(events[1].kind, EventKind::CapabilityRevoked);
    assert_eq!(events[1].agent, agent);
    assert_eq!(events[1].resource, Some(resource));
    assert_eq!(events[1].capability, Some(capability));
    assert_eq!(events[1].operations, operations);
    assert_eq!(events[1].source_capability, None);
    assert_eq!(
        core.authorize(agent, capability, resource, Operation::Observe),
        Err(KernelError::CapabilityRevoked)
    );
    assert_eq!(core.events().len(), 2);
}

#[test]
fn revoke_capability_returns_event_log_full_without_revoking() {
    let mut core = KernelCore::<1, 1, 1, 0, 0, 0>::new();
    let agent = AgentId::new(4);
    let resource = core
        .register_resource(ResourceKind::Workspace, None)
        .expect("resource should fit");
    let capability = core
        .grant_capability(agent, resource, OperationSet::only(Operation::Observe))
        .expect("grant should consume only event slot");

    let result = core.revoke_capability(capability);

    assert_eq!(result, Err(KernelError::EventLogFull));
    assert_eq!(
        core.authorize(agent, capability, resource, Operation::Act),
        Err(KernelError::OperationDenied)
    );
    assert_eq!(core.events().len(), 1);
}

#[test]
fn delegate_task_records_capability_derived_before_delegation() {
    let mut core = TestCore::new();
    let owner = AgentId::new(5);
    let assignee = AgentId::new(6);
    let resource = core
        .register_resource(ResourceKind::Workspace, None)
        .expect("resource should fit");
    let owner_capability = core
        .grant_capability(
            owner,
            resource,
            OperationSet::empty()
                .with(Operation::Act)
                .with(Operation::Delegate),
        )
        .expect("owner capability should fit");
    let intent = declare_action_intent(&mut core, owner, owner_capability, resource);
    let task = core
        .create_task(owner, owner_capability, intent)
        .expect("task should be created");

    let delegation = core
        .delegate_task(owner, owner_capability, task, assignee)
        .expect("task should delegate");
    let derived = core.tasks()[0]
        .delegated_capability
        .expect("delegation should derive capability");

    let events = core.events();
    assert_eq!(events[3].kind, EventKind::IntentBound);
    assert_eq!(events[3].task, Some(task));
    assert_eq!(events[3].intent, Some(intent));
    assert_eq!(events[4].kind, EventKind::CapabilityDerived);
    assert_eq!(events[4].agent, owner);
    assert_eq!(events[4].target_agent, Some(assignee));
    assert_eq!(events[4].resource, Some(resource));
    assert_eq!(events[4].capability, Some(derived));
    assert_eq!(events[4].source_capability, Some(owner_capability));
    assert_eq!(events[4].operations, OperationSet::only(Operation::Act));
    assert_eq!(events[4].task, Some(task));
    assert_eq!(events[4].intent, Some(intent));
    assert_eq!(events[5].kind, EventKind::DelegationRequested);
    assert_eq!(delegation.capability, Some(derived));
}

#[test]
fn delegate_task_requires_two_event_slots_for_derive_and_delegation() {
    let mut core = KernelCore::<1, 4, 4, 1, 2, 2>::new();
    let owner = AgentId::new(7);
    let assignee = AgentId::new(8);
    let resource = core
        .register_resource(ResourceKind::Workspace, None)
        .expect("resource should fit");
    let owner_capability = core
        .grant_capability(
            owner,
            resource,
            OperationSet::empty()
                .with(Operation::Act)
                .with(Operation::Delegate),
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
    let events_after_create = core.events().len();

    let result = core.delegate_task(owner, owner_capability, task, assignee);

    assert_eq!(result, Err(KernelError::EventLogFull));
    assert_eq!(core.tasks()[0].status, TaskStatus::Created);
    assert_eq!(core.tasks()[0].assignee, None);
    assert_eq!(core.tasks()[0].delegated_capability, None);
    assert_eq!(core.events().len(), events_after_create);
}
