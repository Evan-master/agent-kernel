use agent_kernel::AgentKernel;
use agent_kernel_core::{
    ActionId, AgentId, CheckpointId, EventKind, Operation, OperationSet, ResourceKind,
};

#[test]
fn kernel_starts_with_empty_event_log() {
    let kernel = AgentKernel::<4, 4, 8>::new();

    assert!(kernel.events().is_empty());
}

#[test]
fn observe_syscall_records_observation_event() {
    let mut kernel = AgentKernel::<4, 4, 8>::new();
    let agent = AgentId::new(42);
    let resource = kernel
        .sys_register_resource(ResourceKind::Workspace, None)
        .expect("resource should fit");
    let capability = kernel
        .sys_grant(agent, resource, OperationSet::only(Operation::Observe))
        .expect("capability should fit");

    let event = kernel
        .sys_observe(agent, capability, resource)
        .expect("observe should be authorized");

    assert_eq!(event.kind, EventKind::Observation);
    assert_eq!(event.agent, agent);
    assert_eq!(event.resource, Some(resource));
    assert_eq!(kernel.events().len(), 1);
}

#[test]
fn checkpoint_and_rollback_syscalls_record_kernel_events() {
    let mut kernel = AgentKernel::<4, 4, 8>::new();
    let agent = AgentId::new(77);
    let checkpoint = CheckpointId::new(5);
    let resource = kernel
        .sys_register_resource(ResourceKind::Workspace, None)
        .expect("resource should fit");
    let capability = kernel
        .sys_grant(
            agent,
            resource,
            OperationSet::empty()
                .with(Operation::Checkpoint)
                .with(Operation::Rollback),
        )
        .expect("capability should fit");

    kernel
        .sys_checkpoint(agent, capability, checkpoint, resource)
        .expect("checkpoint event should fit");
    kernel
        .sys_rollback(agent, capability, checkpoint, resource)
        .expect("rollback event should fit");

    let events = kernel.events();
    assert_eq!(events.len(), 2);
    assert_eq!(events[0].kind, EventKind::CheckpointCreated);
    assert_eq!(events[1].kind, EventKind::RollbackRequested);
}

#[test]
fn action_and_verify_syscalls_record_action_lifecycle() {
    let mut kernel = AgentKernel::<4, 4, 8>::new();
    let agent = AgentId::new(88);
    let action = ActionId::new(3);
    let resource = kernel
        .sys_register_resource(ResourceKind::Workspace, None)
        .expect("resource should fit");
    let capability = kernel
        .sys_grant(
            agent,
            resource,
            OperationSet::empty()
                .with(Operation::Act)
                .with(Operation::Verify),
        )
        .expect("capability should fit");

    kernel
        .sys_act(agent, capability, action, resource)
        .expect("act should be authorized");
    kernel
        .sys_verify(agent, capability, action, resource)
        .expect("verify should be authorized");

    let events = kernel.events();
    assert_eq!(events.len(), 2);
    assert_eq!(events[0].kind, EventKind::ActionExecuted);
    assert_eq!(events[1].kind, EventKind::VerificationRequested);
    assert_eq!(events[0].action, Some(action));
    assert_eq!(events[1].action, Some(action));
}
