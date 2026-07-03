use agent_kernel_core::{
    AgentId, CheckpointId, CheckpointStatus, EventKind, KernelCore, KernelError, Operation,
    OperationSet, ResourceKind,
};

type TestCore = KernelCore<4, 4, 16, 2, 2, 4, 0, 0, 0>;

#[test]
fn checkpoint_records_checkpoint_and_event() {
    let mut core = TestCore::new();
    let agent = AgentId::new(1);
    let resource = core
        .register_resource(ResourceKind::Workspace, None)
        .expect("resource should fit");
    let capability = core
        .grant_capability(agent, resource, OperationSet::only(Operation::Checkpoint))
        .expect("capability should fit");
    let checkpoint = CheckpointId::new(7);
    let events_after_grant = core.events().len();

    let event = core
        .checkpoint(agent, capability, checkpoint, resource)
        .expect("checkpoint should record");

    assert_eq!(core.checkpoints().len(), 1);
    assert_eq!(core.checkpoints()[0].id, checkpoint);
    assert_eq!(core.checkpoints()[0].agent, agent);
    assert_eq!(core.checkpoints()[0].resource, resource);
    assert_eq!(core.checkpoints()[0].capability, capability);
    assert_eq!(core.checkpoints()[0].status, CheckpointStatus::Created);
    assert_eq!(event.kind, EventKind::CheckpointCreated);
    assert_eq!(event.checkpoint, Some(checkpoint));
    assert_eq!(event.operation, Some(Operation::Checkpoint));
    assert_eq!(core.events().len(), events_after_grant + 1);
    assert_eq!(
        core.events()[events_after_grant].kind,
        EventKind::CheckpointCreated
    );
    assert_eq!(
        core.events()[events_after_grant].checkpoint,
        Some(checkpoint)
    );
}

#[test]
fn checkpoint_rejects_duplicate_without_event() {
    let mut core = TestCore::new();
    let agent = AgentId::new(2);
    let resource = core
        .register_resource(ResourceKind::Workspace, None)
        .expect("resource should fit");
    let capability = core
        .grant_capability(agent, resource, OperationSet::only(Operation::Checkpoint))
        .expect("capability should fit");
    let checkpoint = CheckpointId::new(8);
    core.checkpoint(agent, capability, checkpoint, resource)
        .expect("first checkpoint should record");
    let events_after_checkpoint = core.events().len();

    let result = core.checkpoint(agent, capability, checkpoint, resource);

    assert_eq!(result, Err(KernelError::CheckpointAlreadyExists));
    assert_eq!(core.checkpoints().len(), 1);
    assert_eq!(core.checkpoints()[0].status, CheckpointStatus::Created);
    assert_eq!(core.events().len(), events_after_checkpoint);
}

#[test]
fn checkpoint_requires_checkpoint_operation_without_mutation() {
    let mut core = TestCore::new();
    let agent = AgentId::new(3);
    let resource = core
        .register_resource(ResourceKind::Workspace, None)
        .expect("resource should fit");
    let capability = core
        .grant_capability(agent, resource, OperationSet::only(Operation::Observe))
        .expect("capability should fit");
    let events_after_grant = core.events().len();
    let grant_event = core.events()[events_after_grant - 1];

    let result = core.checkpoint(agent, capability, CheckpointId::new(9), resource);

    assert_eq!(result, Err(KernelError::OperationDenied));
    assert!(core.checkpoints().is_empty());
    assert_eq!(core.events().len(), events_after_grant);
    assert_eq!(core.events()[events_after_grant - 1], grant_event);
}

#[test]
fn checkpoint_store_full_leaves_events_unchanged() {
    let mut core = KernelCore::<1, 1, 4, 1, 1, 0, 0, 0, 0>::new();
    let agent = AgentId::new(4);
    let resource = core
        .register_resource(ResourceKind::Workspace, None)
        .expect("resource should fit");
    let capability = core
        .grant_capability(agent, resource, OperationSet::only(Operation::Checkpoint))
        .expect("capability should fit");
    let events_after_grant = core.events().len();
    let grant_event = core.events()[events_after_grant - 1];

    let result = core.checkpoint(agent, capability, CheckpointId::new(10), resource);

    assert_eq!(result, Err(KernelError::CheckpointStoreFull));
    assert!(core.checkpoints().is_empty());
    assert_eq!(core.events().len(), events_after_grant);
    assert_eq!(core.events()[events_after_grant - 1], grant_event);
}

#[test]
fn checkpoint_event_log_full_leaves_checkpoints_unchanged() {
    let mut core = KernelCore::<1, 1, 1, 1, 1, 1, 0, 0, 0>::new();
    let agent = AgentId::new(5);
    let resource = core
        .register_resource(ResourceKind::Workspace, None)
        .expect("resource should fit");
    let capability = core
        .grant_capability(agent, resource, OperationSet::only(Operation::Checkpoint))
        .expect("grant should consume only event slot");
    let grant_event = core.events()[0];

    let result = core.checkpoint(agent, capability, CheckpointId::new(11), resource);

    assert_eq!(result, Err(KernelError::EventLogFull));
    assert!(core.checkpoints().is_empty());
    assert_eq!(core.events().len(), 1);
    assert_eq!(core.events()[0], grant_event);
}
