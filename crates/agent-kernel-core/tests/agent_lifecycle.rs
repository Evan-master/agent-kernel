use agent_kernel_core::{
    ActionId, AgentEntryKind, AgentId, AgentStatus, EventKind, IntentKind, KernelCore, KernelError,
    Operation, OperationSet, ResourceKind, TaskStatus, VerificationRequirement,
};

type TestCore = KernelCore<2, 1, 2, 16, 1, 1, 1, 1, 1, 1>;

#[test]
fn suspend_agent_records_status_and_event() {
    let mut core = TestCore::new();
    let agent = AgentId::new(1);
    core.register_agent(agent)
        .expect("agent registration should fit");

    let event = core.suspend_agent(agent).expect("suspend event should fit");

    assert_eq!(core.agents()[0].status, AgentStatus::Suspended);
    assert_eq!(event.kind, EventKind::AgentSuspended);
    assert_eq!(event.agent, agent);
    assert_eq!(event.target_agent, Some(agent));
    assert_eq!(core.events().len(), 2);
}

#[test]
fn suspend_agent_event_log_full_leaves_status_active() {
    let mut core = KernelCore::<1, 1, 1, 1, 0, 0, 0, 0, 0, 0>::new();
    let agent = AgentId::new(8);
    core.register_agent(agent)
        .expect("agent registration should consume the only event slot");

    assert_eq!(core.suspend_agent(agent), Err(KernelError::EventLogFull));
    assert_eq!(core.agents()[0].status, AgentStatus::Active);
    assert_eq!(core.events().len(), 1);
}

#[test]
fn resume_agent_reactivates_suspended_agent_and_records_event() {
    let mut core = TestCore::new();
    let agent = AgentId::new(2);
    core.register_agent(agent)
        .expect("agent registration should fit");
    core.suspend_agent(agent).expect("agent should suspend");

    let event = core.resume_agent(agent).expect("resume event should fit");

    assert_eq!(core.agents()[0].status, AgentStatus::Active);
    assert_eq!(event.kind, EventKind::AgentResumed);
    assert_eq!(event.agent, agent);
    assert_eq!(event.target_agent, Some(agent));
}

#[test]
fn retire_agent_records_terminal_status_and_blocks_resume() {
    let mut core = TestCore::new();
    let agent = AgentId::new(3);
    core.register_agent(agent)
        .expect("agent registration should fit");

    let event = core.retire_agent(agent).expect("retire event should fit");
    let events_after_retire = core.events().len();

    assert_eq!(core.agents()[0].status, AgentStatus::Retired);
    assert_eq!(event.kind, EventKind::AgentRetired);
    assert_eq!(event.target_agent, Some(agent));
    assert_eq!(core.resume_agent(agent), Err(KernelError::AgentRetired));
    assert_eq!(core.events().len(), events_after_retire);
}

#[test]
fn suspended_agent_cannot_receive_or_use_authority_without_mutation() {
    let mut core = TestCore::new();
    let agent = AgentId::new(4);
    core.register_agent(agent)
        .expect("agent registration should fit");
    let resource = core
        .register_resource(ResourceKind::Workspace, None)
        .expect("resource should fit");
    let capability = core
        .grant_capability(
            agent,
            resource,
            OperationSet::empty()
                .with(Operation::Observe)
                .with(Operation::Act),
        )
        .expect("capability should fit");
    core.suspend_agent(agent).expect("agent should suspend");
    let events_after_suspend = core.events().len();

    assert_eq!(
        core.grant_capability(agent, resource, OperationSet::only(Operation::Observe)),
        Err(KernelError::AgentSuspended)
    );
    assert_eq!(
        core.observe(agent, capability, resource),
        Err(KernelError::AgentSuspended)
    );
    assert_eq!(
        core.act(agent, capability, ActionId::new(1), resource),
        Err(KernelError::AgentSuspended)
    );
    assert!(core.observations().is_empty());
    assert!(core.actions().is_empty());
    assert_eq!(core.events().len(), events_after_suspend);
}

#[test]
fn retired_agent_cannot_receive_or_use_authority_without_mutation() {
    let mut core = TestCore::new();
    let agent = AgentId::new(5);
    core.register_agent(agent)
        .expect("agent registration should fit");
    let resource = core
        .register_resource(ResourceKind::Workspace, None)
        .expect("resource should fit");
    let capability = core
        .grant_capability(agent, resource, OperationSet::only(Operation::Observe))
        .expect("capability should fit");
    core.retire_agent(agent).expect("agent should retire");
    let events_after_retire = core.events().len();

    assert_eq!(
        core.grant_capability(agent, resource, OperationSet::only(Operation::Observe)),
        Err(KernelError::AgentRetired)
    );
    assert_eq!(
        core.observe(agent, capability, resource),
        Err(KernelError::AgentRetired)
    );
    assert!(core.observations().is_empty());
    assert_eq!(core.events().len(), events_after_retire);
}

#[test]
fn suspended_parent_agent_invalidates_delegated_task_authority() {
    let mut core = TestCore::new();
    let owner = AgentId::new(6);
    let assignee = AgentId::new(7);
    core.register_agent(owner)
        .expect("owner registration should fit");
    core.register_agent(assignee)
        .expect("assignee registration should fit");
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
        .expect("intent should fit");
    let task = core
        .create_task(owner, owner_capability, intent)
        .expect("task should fit");
    core.delegate_task(owner, owner_capability, task, assignee)
        .expect("task should delegate");
    let assignee_capability = core.tasks()[0]
        .delegated_capability
        .expect("delegation should derive capability");
    core.launch_task_agent(assignee, assignee_capability, task, AgentEntryKind::Worker)
        .expect("assignee should launch for delegated task");
    core.accept_task(assignee, task)
        .expect("task should accept");
    core.enqueue_task(assignee, task)
        .expect("task should enqueue");
    core.dispatch_next(assignee).expect("task should dispatch");
    core.suspend_agent(owner).expect("owner should suspend");
    let events_after_suspend = core.events().len();

    assert_eq!(
        core.complete_task(assignee, assignee_capability, task),
        Err(KernelError::AgentSuspended)
    );
    assert_eq!(core.tasks()[0].status, TaskStatus::Running);
    assert_eq!(core.events().len(), events_after_suspend);
}
