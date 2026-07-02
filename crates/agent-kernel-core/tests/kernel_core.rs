use agent_kernel_core::{
    ActionId, AgentId, CheckpointId, EventKind, KernelCore, Operation, OperationSet, ResourceKind,
};

type TestCore = KernelCore<4, 4, 16, 4>;

#[test]
fn observes_resource_when_capability_allows_observe() {
    let mut core = TestCore::new();
    let agent = AgentId::new(7);
    let resource = core
        .register_resource(ResourceKind::Workspace, None)
        .expect("resource should fit");
    let capability = core
        .grant_capability(agent, resource, OperationSet::only(Operation::Observe))
        .expect("capability should fit");

    let event = core
        .authorize(agent, capability, resource, Operation::Observe)
        .expect("observe should be authorized");

    assert_eq!(event.agent, agent);
    assert_eq!(event.resource, Some(resource));
    assert_eq!(event.kind, EventKind::Observation);
    assert_eq!(core.events().len(), 1);
}

#[test]
fn denies_action_when_capability_does_not_include_operation() {
    let mut core = TestCore::new();
    let agent = AgentId::new(1);
    let resource = core
        .register_resource(ResourceKind::Workspace, None)
        .expect("resource should fit");
    let capability = core
        .grant_capability(agent, resource, OperationSet::only(Operation::Observe))
        .expect("capability should fit");

    let result = core.authorize(agent, capability, resource, Operation::Act);

    assert!(result.is_err());
    assert_eq!(core.events().len(), 0);
}

#[test]
fn revoked_capability_can_no_longer_authorize_operation() {
    let mut core = TestCore::new();
    let agent = AgentId::new(2);
    let resource = core
        .register_resource(ResourceKind::Service, None)
        .expect("resource should fit");
    let capability = core
        .grant_capability(agent, resource, OperationSet::only(Operation::Observe))
        .expect("capability should fit");

    core.revoke_capability(capability)
        .expect("capability should exist");

    assert!(core
        .authorize(agent, capability, resource, Operation::Observe)
        .is_err());
    assert_eq!(core.events().len(), 0);
}

#[test]
fn checkpoint_and_rollback_events_are_recorded_in_order() {
    let mut core = TestCore::new();
    let agent = AgentId::new(3);
    let resource = core
        .register_resource(ResourceKind::Workspace, None)
        .expect("resource should fit");
    let capability = core
        .grant_capability(
            agent,
            resource,
            OperationSet::empty()
                .with(Operation::Checkpoint)
                .with(Operation::Rollback),
        )
        .expect("capability should fit");
    let checkpoint = CheckpointId::new(9);

    core.checkpoint(agent, capability, checkpoint, resource)
        .expect("checkpoint event should fit");
    core.rollback(agent, capability, checkpoint, resource)
        .expect("rollback event should fit");

    let events = core.events();
    assert_eq!(events.len(), 2);
    assert_eq!(events[0].kind, EventKind::CheckpointCreated);
    assert_eq!(events[1].kind, EventKind::RollbackRequested);
    assert_eq!(events[0].checkpoint, Some(checkpoint));
    assert_eq!(events[1].checkpoint, Some(checkpoint));
}

#[test]
fn checkpoint_requires_checkpoint_capability() {
    let mut core = TestCore::new();
    let agent = AgentId::new(4);
    let resource = core
        .register_resource(ResourceKind::Workspace, None)
        .expect("resource should fit");
    let capability = core
        .grant_capability(agent, resource, OperationSet::only(Operation::Observe))
        .expect("capability should fit");

    let result = core.checkpoint(agent, capability, CheckpointId::new(10), resource);

    assert!(result.is_err());
    assert_eq!(core.events().len(), 0);
}

#[test]
fn action_and_verification_events_are_recorded_with_action_id() {
    let mut core = TestCore::new();
    let agent = AgentId::new(5);
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
    let action = ActionId::new(12);

    core.act(agent, capability, action, resource)
        .expect("act event should fit");
    core.verify(agent, capability, action, resource)
        .expect("verify event should fit");

    let events = core.events();
    assert_eq!(events.len(), 2);
    assert_eq!(events[0].kind, EventKind::ActionExecuted);
    assert_eq!(events[1].kind, EventKind::VerificationRequested);
    assert_eq!(events[0].action, Some(action));
    assert_eq!(events[1].action, Some(action));
}

#[test]
fn action_requires_action_capability() {
    let mut core = TestCore::new();
    let agent = AgentId::new(6);
    let resource = core
        .register_resource(ResourceKind::Workspace, None)
        .expect("resource should fit");
    let capability = core
        .grant_capability(agent, resource, OperationSet::only(Operation::Observe))
        .expect("capability should fit");

    let result = core.act(agent, capability, ActionId::new(13), resource);

    assert!(result.is_err());
    assert_eq!(core.events().len(), 0);
}
