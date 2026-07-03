use agent_kernel_core::{
    ActionId, ActionStatus, AgentId, EventKind, KernelCore, KernelError, ObservationId, Operation,
    OperationSet, ResourceKind,
};

type TestCore = KernelCore<4, 4, 16, 4, 4, 2, 0, 0, 0>;

#[test]
fn observe_records_observation_and_event() {
    let mut core = TestCore::new();
    let agent = AgentId::new(1);
    let resource = core
        .register_resource(ResourceKind::Workspace, None)
        .expect("resource should fit");
    let capability = core
        .grant_capability(agent, resource, OperationSet::only(Operation::Observe))
        .expect("capability should fit");
    let events_after_grant = core.events().len();

    let event = core
        .observe(agent, capability, resource)
        .expect("observation should record");

    assert_eq!(core.observations().len(), 1);
    assert_eq!(core.observations()[0].id, ObservationId::new(1));
    assert_eq!(core.observations()[0].agent, agent);
    assert_eq!(core.observations()[0].resource, resource);
    assert_eq!(core.observations()[0].capability, capability);
    assert_eq!(event.kind, EventKind::Observation);
    assert_eq!(event.observation, Some(ObservationId::new(1)));
    assert_eq!(event.resource, Some(resource));
    assert_eq!(event.capability, Some(capability));
    assert_eq!(event.operation, Some(Operation::Observe));
    assert_eq!(core.events().len(), events_after_grant + 1);
    assert_eq!(
        core.events()[events_after_grant].kind,
        EventKind::Observation
    );
    assert_eq!(
        core.events()[events_after_grant].observation,
        Some(ObservationId::new(1))
    );
    assert_eq!(core.events()[events_after_grant].resource, Some(resource));
    assert_eq!(
        core.events()[events_after_grant].capability,
        Some(capability)
    );
    assert_eq!(
        core.events()[events_after_grant].operation,
        Some(Operation::Observe)
    );
}

#[test]
fn observe_store_full_leaves_events_unchanged() {
    let mut core = KernelCore::<1, 1, 4, 1, 0, 2, 0, 0, 0>::new();
    let agent = AgentId::new(2);
    let resource = core
        .register_resource(ResourceKind::Workspace, None)
        .expect("resource should fit");
    let capability = core
        .grant_capability(agent, resource, OperationSet::only(Operation::Observe))
        .expect("capability should fit");
    let events_after_grant = core.events().len();
    let grant_event = core.events()[events_after_grant - 1];

    let result = core.observe(agent, capability, resource);

    assert_eq!(result, Err(KernelError::ObservationStoreFull));
    assert!(core.observations().is_empty());
    assert_eq!(core.events().len(), events_after_grant);
    assert_eq!(core.events()[events_after_grant - 1], grant_event);
}

#[test]
fn observe_event_log_full_leaves_observations_unchanged() {
    let mut core = KernelCore::<1, 1, 1, 1, 1, 2, 0, 0, 0>::new();
    let agent = AgentId::new(3);
    let resource = core
        .register_resource(ResourceKind::Workspace, None)
        .expect("resource should fit");
    let capability = core
        .grant_capability(agent, resource, OperationSet::only(Operation::Observe))
        .expect("grant should consume only event slot");
    let grant_event = core.events()[0];

    let result = core.observe(agent, capability, resource);

    assert_eq!(result, Err(KernelError::EventLogFull));
    assert!(core.observations().is_empty());
    assert_eq!(core.events().len(), 1);
    assert_eq!(core.events()[0], grant_event);
}

#[test]
fn act_records_action_and_event() {
    let mut core = TestCore::new();
    let agent = AgentId::new(4);
    let resource = core
        .register_resource(ResourceKind::Workspace, None)
        .expect("resource should fit");
    let capability = core
        .grant_capability(agent, resource, OperationSet::only(Operation::Act))
        .expect("capability should fit");
    let action = ActionId::new(9);
    let events_after_grant = core.events().len();

    let event = core
        .act(agent, capability, action, resource)
        .expect("action should record");

    assert_eq!(core.actions().len(), 1);
    assert_eq!(core.actions()[0].id, action);
    assert_eq!(core.actions()[0].agent, agent);
    assert_eq!(core.actions()[0].resource, resource);
    assert_eq!(core.actions()[0].capability, capability);
    assert_eq!(core.actions()[0].status, ActionStatus::Executed);
    assert_eq!(event.kind, EventKind::ActionExecuted);
    assert_eq!(event.action, Some(action));
    assert_eq!(event.observation, None);
    assert_eq!(core.events().len(), events_after_grant + 1);
    assert_eq!(
        core.events()[events_after_grant].kind,
        EventKind::ActionExecuted
    );
    assert_eq!(core.events()[events_after_grant].action, Some(action));
    assert_eq!(core.events()[events_after_grant].resource, Some(resource));
    assert_eq!(
        core.events()[events_after_grant].capability,
        Some(capability)
    );
}

#[test]
fn act_rejects_duplicate_action_without_event() {
    let mut core = TestCore::new();
    let agent = AgentId::new(5);
    let resource = core
        .register_resource(ResourceKind::Workspace, None)
        .expect("resource should fit");
    let capability = core
        .grant_capability(agent, resource, OperationSet::only(Operation::Act))
        .expect("capability should fit");
    let action = ActionId::new(10);
    core.act(agent, capability, action, resource)
        .expect("first action should record");
    let events_after_first = core.events().len();
    let grant_event = core.events()[0];
    let action_event = core.events()[events_after_first - 1];

    let result = core.act(agent, capability, action, resource);

    assert_eq!(result, Err(KernelError::ActionAlreadyExists));
    assert_eq!(core.actions().len(), 1);
    assert_eq!(core.actions()[0].status, ActionStatus::Executed);
    assert_eq!(core.events().len(), events_after_first);
    assert_eq!(core.events()[0], grant_event);
    assert_eq!(core.events()[events_after_first - 1], action_event);
}

#[test]
fn act_store_full_leaves_events_unchanged() {
    let mut core = KernelCore::<1, 1, 4, 0, 1, 2, 0, 0, 0>::new();
    let agent = AgentId::new(6);
    let resource = core
        .register_resource(ResourceKind::Workspace, None)
        .expect("resource should fit");
    let capability = core
        .grant_capability(agent, resource, OperationSet::only(Operation::Act))
        .expect("capability should fit");
    let events_after_grant = core.events().len();
    let grant_event = core.events()[events_after_grant - 1];

    let result = core.act(agent, capability, ActionId::new(11), resource);

    assert_eq!(result, Err(KernelError::ActionStoreFull));
    assert!(core.actions().is_empty());
    assert_eq!(core.events().len(), events_after_grant);
    assert_eq!(core.events()[events_after_grant - 1], grant_event);
}

#[test]
fn act_event_log_full_leaves_actions_unchanged() {
    let mut core = KernelCore::<1, 1, 1, 1, 1, 2, 0, 0, 0>::new();
    let agent = AgentId::new(7);
    let resource = core
        .register_resource(ResourceKind::Workspace, None)
        .expect("resource should fit");
    let capability = core
        .grant_capability(agent, resource, OperationSet::only(Operation::Act))
        .expect("grant should consume only event slot");
    let grant_event = core.events()[0];

    let result = core.act(agent, capability, ActionId::new(12), resource);

    assert_eq!(result, Err(KernelError::EventLogFull));
    assert!(core.actions().is_empty());
    assert_eq!(core.events().len(), 1);
    assert_eq!(core.events()[0], grant_event);
}

#[test]
fn verify_existing_action_updates_status_and_event() {
    let mut core = TestCore::new();
    let agent = AgentId::new(8);
    let resource = core
        .register_resource(ResourceKind::Workspace, None)
        .expect("resource should fit");
    let capability = core
        .grant_capability(
            agent,
            resource,
            OperationSet::empty()
                .with(Operation::Act)
                .with(Operation::Verify),
        )
        .expect("capability should fit");
    let action = ActionId::new(13);
    core.act(agent, capability, action, resource)
        .expect("action should record");
    let events_after_action = core.events().len();

    let event = core
        .verify(agent, capability, action, resource)
        .expect("verification should record");

    assert_eq!(
        core.actions()[0].status,
        ActionStatus::VerificationRequested
    );
    assert_eq!(event.kind, EventKind::VerificationRequested);
    assert_eq!(event.action, Some(action));
    assert_eq!(event.resource, Some(resource));
    assert_eq!(core.events().len(), events_after_action + 1);
    assert_eq!(
        core.events()[events_after_action].kind,
        EventKind::VerificationRequested
    );
    assert_eq!(core.events()[events_after_action].action, Some(action));
    assert_eq!(core.events()[events_after_action].resource, Some(resource));
    assert_eq!(
        core.events()[events_after_action].capability,
        Some(capability)
    );
}

#[test]
fn verify_missing_action_leaves_events_unchanged() {
    let mut core = TestCore::new();
    let agent = AgentId::new(9);
    let resource = core
        .register_resource(ResourceKind::Workspace, None)
        .expect("resource should fit");
    let capability = core
        .grant_capability(agent, resource, OperationSet::only(Operation::Verify))
        .expect("capability should fit");
    let events_after_grant = core.events().len();
    let grant_event = core.events()[events_after_grant - 1];

    let result = core.verify(agent, capability, ActionId::new(14), resource);

    assert_eq!(result, Err(KernelError::ActionNotFound));
    assert!(core.actions().is_empty());
    assert_eq!(core.events().len(), events_after_grant);
    assert_eq!(core.events()[events_after_grant - 1], grant_event);
}

#[test]
fn verify_rejects_action_resource_mismatch_without_status_change() {
    let mut core = KernelCore::<2, 2, 8, 2, 1, 2, 0, 0, 0>::new();
    let agent = AgentId::new(10);
    let first_resource = core
        .register_resource(ResourceKind::Workspace, None)
        .expect("first resource should fit");
    let second_resource = core
        .register_resource(ResourceKind::Service, None)
        .expect("second resource should fit");
    let act_capability = core
        .grant_capability(agent, first_resource, OperationSet::only(Operation::Act))
        .expect("act capability should fit");
    let verify_capability = core
        .grant_capability(
            agent,
            second_resource,
            OperationSet::only(Operation::Verify),
        )
        .expect("verify capability should fit");
    let action = ActionId::new(15);
    core.act(agent, act_capability, action, first_resource)
        .expect("action should record");
    let events_after_action = core.events().len();
    let first_grant_event = core.events()[0];
    let second_grant_event = core.events()[1];
    let action_event = core.events()[events_after_action - 1];
    let actions_after_action = core.actions().len();
    let action_id_before = core.actions()[0].id;
    let action_agent_before = core.actions()[0].agent;
    let action_resource_before = core.actions()[0].resource;
    let action_capability_before = core.actions()[0].capability;
    let action_status_before = core.actions()[0].status;

    let result = core.verify(agent, verify_capability, action, second_resource);

    assert_eq!(result, Err(KernelError::ActionResourceMismatch));
    assert_eq!(core.actions().len(), actions_after_action);
    assert_eq!(core.actions()[0].id, action_id_before);
    assert_eq!(core.actions()[0].agent, action_agent_before);
    assert_eq!(core.actions()[0].resource, action_resource_before);
    assert_eq!(core.actions()[0].capability, action_capability_before);
    assert_eq!(core.actions()[0].status, action_status_before);
    assert_eq!(core.events().len(), events_after_action);
    assert_eq!(core.events()[0], first_grant_event);
    assert_eq!(core.events()[1], second_grant_event);
    assert_eq!(core.events()[events_after_action - 1], action_event);
}

#[test]
fn verify_rejects_repeated_verification_without_event() {
    let mut core = TestCore::new();
    let agent = AgentId::new(11);
    let resource = core
        .register_resource(ResourceKind::Workspace, None)
        .expect("resource should fit");
    let capability = core
        .grant_capability(
            agent,
            resource,
            OperationSet::empty()
                .with(Operation::Act)
                .with(Operation::Verify),
        )
        .expect("capability should fit");
    let action = ActionId::new(16);
    core.act(agent, capability, action, resource)
        .expect("action should record");
    core.verify(agent, capability, action, resource)
        .expect("first verification should record");
    let events_after_verify = core.events().len();
    let grant_event = core.events()[0];
    let action_event = core.events()[1];
    let verify_event = core.events()[events_after_verify - 1];
    let actions_after_verify = core.actions().len();
    let action_id_before = core.actions()[0].id;
    let action_agent_before = core.actions()[0].agent;
    let action_resource_before = core.actions()[0].resource;
    let action_capability_before = core.actions()[0].capability;
    let action_status_before = core.actions()[0].status;

    let result = core.verify(agent, capability, action, resource);

    assert_eq!(result, Err(KernelError::ActionStatusMismatch));
    assert_eq!(core.actions().len(), actions_after_verify);
    assert_eq!(core.actions()[0].id, action_id_before);
    assert_eq!(core.actions()[0].agent, action_agent_before);
    assert_eq!(core.actions()[0].resource, action_resource_before);
    assert_eq!(core.actions()[0].capability, action_capability_before);
    assert_eq!(core.actions()[0].status, action_status_before);
    assert_eq!(core.events().len(), events_after_verify);
    assert_eq!(core.events()[0], grant_event);
    assert_eq!(core.events()[1], action_event);
    assert_eq!(core.events()[events_after_verify - 1], verify_event);
}

#[test]
fn verify_event_log_full_leaves_action_status_executed() {
    let mut core = KernelCore::<1, 1, 2, 1, 1, 2, 0, 0, 0>::new();
    let agent = AgentId::new(12);
    let resource = core
        .register_resource(ResourceKind::Workspace, None)
        .expect("resource should fit");
    let capability = core
        .grant_capability(
            agent,
            resource,
            OperationSet::empty()
                .with(Operation::Act)
                .with(Operation::Verify),
        )
        .expect("grant should consume first event");
    let action = ActionId::new(17);
    core.act(agent, capability, action, resource)
        .expect("act should consume second event");
    let events_after_action = core.events().len();
    let grant_event = core.events()[0];
    let action_event = core.events()[events_after_action - 1];
    let actions_after_action = core.actions().len();
    let action_id_before = core.actions()[0].id;
    let action_agent_before = core.actions()[0].agent;
    let action_resource_before = core.actions()[0].resource;
    let action_capability_before = core.actions()[0].capability;
    let action_status_before = core.actions()[0].status;

    let result = core.verify(agent, capability, action, resource);

    assert_eq!(result, Err(KernelError::EventLogFull));
    assert_eq!(core.actions().len(), actions_after_action);
    assert_eq!(core.actions()[0].id, action_id_before);
    assert_eq!(core.actions()[0].agent, action_agent_before);
    assert_eq!(core.actions()[0].resource, action_resource_before);
    assert_eq!(core.actions()[0].capability, action_capability_before);
    assert_eq!(core.actions()[0].status, action_status_before);
    assert_eq!(core.events().len(), events_after_action);
    assert_eq!(core.events()[0], grant_event);
    assert_eq!(core.events()[events_after_action - 1], action_event);
}
