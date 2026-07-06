use agent_kernel_core::{
    AgentEntryKind, AgentId, CapabilityId, IntentKind, KernelCore, KernelError, Operation,
    OperationSet, ResourceId, ResourceKind, VerificationRequirement,
};

type TestCore = KernelCore<2, 4, 8, 32, 2, 2, 2, 4, 4, 4>;

fn register_with_capability(
    core: &mut TestCore,
    agent: AgentId,
    operations: OperationSet,
) -> (CapabilityId, ResourceId) {
    core.register_agent(agent).expect("agent should register");
    let resource = core
        .register_resource(ResourceKind::Workspace, None)
        .expect("resource should fit");
    let capability = core
        .grant_capability(agent, resource, operations)
        .expect("capability should fit");
    (capability, resource)
}

#[test]
fn launch_rejects_unknown_agent_without_event() {
    let mut core = TestCore::new();
    let result = core.launch_agent(
        AgentId::new(7),
        CapabilityId::new(1),
        ResourceId::new(1),
        AgentEntryKind::Worker,
        None,
    );

    assert_eq!(result, Err(KernelError::AgentNotFound));
    assert!(core.agent_entries().is_empty());
    assert!(core.events().is_empty());
}

#[test]
fn launch_requires_act_authority_without_partial_entry() {
    let mut core = TestCore::new();
    let agent = AgentId::new(1);
    let (capability, resource) =
        register_with_capability(&mut core, agent, OperationSet::only(Operation::Observe));
    let events_after_grant = core.events().len();

    let result = core.launch_agent(
        agent,
        capability,
        resource,
        AgentEntryKind::Supervisor,
        None,
    );

    assert_eq!(result, Err(KernelError::OperationDenied));
    assert!(core.agent_entries().is_empty());
    assert_eq!(core.events().len(), events_after_grant);
}

#[test]
fn launch_rejects_duplicate_entry_without_second_event() {
    let mut core = TestCore::new();
    let agent = AgentId::new(1);
    let (capability, resource) =
        register_with_capability(&mut core, agent, OperationSet::only(Operation::Act));
    core.launch_agent(
        agent,
        capability,
        resource,
        AgentEntryKind::Supervisor,
        None,
    )
    .expect("first launch should succeed");
    let events_after_launch = core.events().len();

    let result = core.launch_agent(
        agent,
        capability,
        resource,
        AgentEntryKind::Supervisor,
        None,
    );

    assert_eq!(result, Err(KernelError::AgentAlreadyLaunched));
    assert_eq!(core.agent_entries().len(), 1);
    assert_eq!(core.events().len(), events_after_launch);
}

#[test]
fn launch_rejects_intent_from_another_agent() {
    let mut core = TestCore::new();
    let owner = AgentId::new(1);
    let other = AgentId::new(2);
    let (owner_capability, resource) =
        register_with_capability(&mut core, owner, OperationSet::only(Operation::Act));
    core.register_agent(other)
        .expect("other agent should register");
    let other_capability = core
        .grant_capability(other, resource, OperationSet::only(Operation::Act))
        .expect("other capability should fit");
    let intent = core
        .declare_intent(
            other,
            other_capability,
            resource,
            IntentKind::Act,
            VerificationRequirement::Required,
        )
        .expect("intent should declare");
    let events_after_intent = core.events().len();

    let result = core.launch_agent(
        owner,
        owner_capability,
        resource,
        AgentEntryKind::Worker,
        Some(intent),
    );

    assert_eq!(result, Err(KernelError::IntentAgentMismatch));
    assert!(core.agent_entries().is_empty());
    assert_eq!(core.events().len(), events_after_intent);
}

#[test]
fn launch_rejects_intent_for_another_resource() {
    let mut core = TestCore::new();
    let agent = AgentId::new(1);
    let (capability, resource) =
        register_with_capability(&mut core, agent, OperationSet::only(Operation::Act));
    let second_resource = core
        .register_resource(ResourceKind::Memory, None)
        .expect("second resource should fit");
    let second_capability = core
        .grant_capability(agent, second_resource, OperationSet::only(Operation::Act))
        .expect("second capability should fit");
    let intent = core
        .declare_intent(
            agent,
            second_capability,
            second_resource,
            IntentKind::Act,
            VerificationRequirement::Required,
        )
        .expect("intent should declare");
    let events_after_intent = core.events().len();

    let result = core.launch_agent(
        agent,
        capability,
        resource,
        AgentEntryKind::Worker,
        Some(intent),
    );

    assert_eq!(result, Err(KernelError::ResourceMismatch));
    assert!(core.agent_entries().is_empty());
    assert_eq!(core.events().len(), events_after_intent);
}

#[test]
fn launch_rejects_non_action_intent() {
    let mut core = TestCore::new();
    let agent = AgentId::new(1);
    let (capability, resource) = register_with_capability(
        &mut core,
        agent,
        OperationSet::empty()
            .with(Operation::Act)
            .with(Operation::Observe),
    );
    let intent = core
        .declare_intent(
            agent,
            capability,
            resource,
            IntentKind::Observe,
            VerificationRequirement::Optional,
        )
        .expect("observe intent should declare");
    let events_after_intent = core.events().len();

    let result = core.launch_agent(
        agent,
        capability,
        resource,
        AgentEntryKind::Worker,
        Some(intent),
    );

    assert_eq!(result, Err(KernelError::IntentKindMismatch));
    assert!(core.agent_entries().is_empty());
    assert_eq!(core.events().len(), events_after_intent);
}

#[test]
fn launch_rejects_bound_intent() {
    let mut core = TestCore::new();
    let agent = AgentId::new(1);
    let (capability, resource) =
        register_with_capability(&mut core, agent, OperationSet::only(Operation::Act));
    let intent = core
        .declare_intent(
            agent,
            capability,
            resource,
            IntentKind::Act,
            VerificationRequirement::Required,
        )
        .expect("intent should declare");
    core.create_task(agent, capability, intent)
        .expect("task should bind intent");
    let events_after_task = core.events().len();

    let result = core.launch_agent(
        agent,
        capability,
        resource,
        AgentEntryKind::Worker,
        Some(intent),
    );

    assert_eq!(result, Err(KernelError::IntentStatusMismatch));
    assert!(core.agent_entries().is_empty());
    assert_eq!(core.events().len(), events_after_task);
}

#[test]
fn launch_event_log_full_leaves_no_entry() {
    let mut core = KernelCore::<1, 1, 1, 2, 0, 0, 0, 0, 0, 0>::new();
    let agent = AgentId::new(1);
    core.register_agent(agent).expect("agent should register");
    let resource = core
        .register_resource(ResourceKind::Workspace, None)
        .expect("resource should fit");
    let capability = core
        .grant_capability(agent, resource, OperationSet::only(Operation::Act))
        .expect("capability should fit");

    let result = core.launch_agent(agent, capability, resource, AgentEntryKind::Bootstrap, None);

    assert_eq!(result, Err(KernelError::EventLogFull));
    assert!(core.agent_entries().is_empty());
    assert_eq!(core.events().len(), 2);
}
