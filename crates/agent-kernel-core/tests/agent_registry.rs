use agent_kernel_core::{AgentId, AgentStatus, EventKind, KernelCore, KernelError};

type TestCore = KernelCore<2, 1, 1, 8, 0, 0, 0, 0, 0, 0>;

#[test]
fn register_agent_records_agent_and_event() {
    let mut core = TestCore::new();
    let agent = AgentId::new(1);

    let event = core
        .register_agent(agent)
        .expect("agent registration should fit");

    assert_eq!(core.agents().len(), 1);
    assert_eq!(core.agents()[0].id, agent);
    assert_eq!(core.agents()[0].status, AgentStatus::Active);
    assert_eq!(event.kind, EventKind::AgentRegistered);
    assert_eq!(event.agent, agent);
    assert_eq!(event.target_agent, Some(agent));
    assert_eq!(event.resource, None);
    assert_eq!(event.capability, None);
    assert_eq!(core.events().len(), 1);
    assert_eq!(core.events()[0], event);
}

#[test]
fn register_agent_rejects_duplicate_without_event() {
    let mut core = TestCore::new();
    let agent = AgentId::new(2);
    core.register_agent(agent)
        .expect("first registration should fit");
    let events_after_registration = core.events().len();

    let result = core.register_agent(agent);

    assert_eq!(result, Err(KernelError::AgentAlreadyExists));
    assert_eq!(core.agents().len(), 1);
    assert_eq!(core.agents()[0].status, AgentStatus::Active);
    assert_eq!(core.events().len(), events_after_registration);
}

#[test]
fn register_agent_store_full_leaves_events_unchanged() {
    let mut core = KernelCore::<0, 1, 1, 4, 0, 0, 0, 0, 0, 0>::new();
    let agent = AgentId::new(3);

    let result = core.register_agent(agent);

    assert_eq!(result, Err(KernelError::AgentStoreFull));
    assert!(core.agents().is_empty());
    assert!(core.events().is_empty());
}

#[test]
fn register_agent_event_log_full_leaves_agents_unchanged() {
    let mut core = KernelCore::<1, 1, 1, 0, 0, 0, 0, 0, 0, 0>::new();
    let agent = AgentId::new(4);

    let result = core.register_agent(agent);

    assert_eq!(result, Err(KernelError::EventLogFull));
    assert!(core.agents().is_empty());
    assert!(core.events().is_empty());
}
