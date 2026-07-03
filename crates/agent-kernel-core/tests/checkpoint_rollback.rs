use agent_kernel_core::{
    AgentId, CheckpointId, CheckpointStatus, EventKind, KernelCore, KernelError, Operation,
    OperationSet, ResourceKind,
};

type TestCore = KernelCore<2, 4, 4, 16, 2, 2, 4, 0, 0, 0>;

#[test]
fn rollback_existing_checkpoint_updates_status_and_event() {
    let mut core = TestCore::new();
    let agent = AgentId::new(6);
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
    let checkpoint = CheckpointId::new(12);
    core.checkpoint(agent, capability, checkpoint, resource)
        .expect("checkpoint should record");
    let events_after_checkpoint = core.events().len();

    let event = core
        .rollback(agent, capability, checkpoint, resource)
        .expect("rollback should record");

    assert_eq!(
        core.checkpoints()[0].status,
        CheckpointStatus::RollbackRequested
    );
    assert_eq!(event.kind, EventKind::RollbackRequested);
    assert_eq!(event.checkpoint, Some(checkpoint));
    assert_eq!(event.operation, Some(Operation::Rollback));
    assert_eq!(core.events().len(), events_after_checkpoint + 1);
    assert_eq!(
        core.events()[events_after_checkpoint].kind,
        EventKind::RollbackRequested
    );
}

#[test]
fn rollback_requires_rollback_operation_without_status_change() {
    let mut core = TestCore::new();
    let agent = AgentId::new(7);
    let resource = core
        .register_resource(ResourceKind::Workspace, None)
        .expect("resource should fit");
    let capability = core
        .grant_capability(agent, resource, OperationSet::only(Operation::Checkpoint))
        .expect("capability should fit");
    let checkpoint = CheckpointId::new(13);
    core.checkpoint(agent, capability, checkpoint, resource)
        .expect("checkpoint should record");
    let events_after_checkpoint = core.events().len();

    let result = core.rollback(agent, capability, checkpoint, resource);

    assert_eq!(result, Err(KernelError::OperationDenied));
    assert_eq!(core.checkpoints().len(), 1);
    assert_eq!(core.checkpoints()[0].status, CheckpointStatus::Created);
    assert_eq!(core.events().len(), events_after_checkpoint);
}

#[test]
fn rollback_missing_checkpoint_leaves_events_unchanged() {
    let mut core = TestCore::new();
    let agent = AgentId::new(8);
    let resource = core
        .register_resource(ResourceKind::Workspace, None)
        .expect("resource should fit");
    let capability = core
        .grant_capability(agent, resource, OperationSet::only(Operation::Rollback))
        .expect("capability should fit");
    let events_after_grant = core.events().len();
    let grant_event = core.events()[events_after_grant - 1];

    let result = core.rollback(agent, capability, CheckpointId::new(14), resource);

    assert_eq!(result, Err(KernelError::CheckpointNotFound));
    assert!(core.checkpoints().is_empty());
    assert_eq!(core.events().len(), events_after_grant);
    assert_eq!(core.events()[events_after_grant - 1], grant_event);
}

#[test]
fn rollback_rejects_checkpoint_resource_mismatch_without_status_change() {
    let mut core = KernelCore::<2, 2, 2, 8, 1, 1, 2, 0, 0, 0>::new();
    let agent = AgentId::new(9);
    let first_resource = core
        .register_resource(ResourceKind::Workspace, None)
        .expect("first resource should fit");
    let second_resource = core
        .register_resource(ResourceKind::Service, None)
        .expect("second resource should fit");
    let checkpoint_capability = core
        .grant_capability(
            agent,
            first_resource,
            OperationSet::only(Operation::Checkpoint),
        )
        .expect("checkpoint capability should fit");
    let rollback_capability = core
        .grant_capability(
            agent,
            second_resource,
            OperationSet::only(Operation::Rollback),
        )
        .expect("rollback capability should fit");
    let checkpoint = CheckpointId::new(15);
    core.checkpoint(agent, checkpoint_capability, checkpoint, first_resource)
        .expect("checkpoint should record");
    let events_after_checkpoint = core.events().len();
    let checkpoint_status_before = core.checkpoints()[0].status;

    let result = core.rollback(agent, rollback_capability, checkpoint, second_resource);

    assert_eq!(result, Err(KernelError::CheckpointResourceMismatch));
    assert_eq!(core.checkpoints().len(), 1);
    assert_eq!(core.checkpoints()[0].status, checkpoint_status_before);
    assert_eq!(core.events().len(), events_after_checkpoint);
}

#[test]
fn rollback_rejects_repeated_request_without_event() {
    let mut core = TestCore::new();
    let agent = AgentId::new(10);
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
    let checkpoint = CheckpointId::new(16);
    core.checkpoint(agent, capability, checkpoint, resource)
        .expect("checkpoint should record");
    core.rollback(agent, capability, checkpoint, resource)
        .expect("first rollback should record");
    let events_after_rollback = core.events().len();

    let result = core.rollback(agent, capability, checkpoint, resource);

    assert_eq!(result, Err(KernelError::CheckpointStatusMismatch));
    assert_eq!(core.checkpoints().len(), 1);
    assert_eq!(
        core.checkpoints()[0].status,
        CheckpointStatus::RollbackRequested
    );
    assert_eq!(core.events().len(), events_after_rollback);
}

#[test]
fn rollback_event_log_full_leaves_checkpoint_status_created() {
    let mut core = KernelCore::<2, 1, 1, 2, 1, 1, 1, 0, 0, 0>::new();
    let agent = AgentId::new(11);
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
        .expect("grant should consume first event");
    let checkpoint = CheckpointId::new(17);
    core.checkpoint(agent, capability, checkpoint, resource)
        .expect("checkpoint should consume second event");
    let events_after_checkpoint = core.events().len();

    let result = core.rollback(agent, capability, checkpoint, resource);

    assert_eq!(result, Err(KernelError::EventLogFull));
    assert_eq!(core.checkpoints().len(), 1);
    assert_eq!(core.checkpoints()[0].status, CheckpointStatus::Created);
    assert_eq!(core.events().len(), events_after_checkpoint);
}
