use agent_kernel_core::{
    AgentId, KernelCore, KernelError, MemoryCellId, MemoryValue, Operation, OperationSet,
    ResourceKind,
};

type TestCore = KernelCore<2, 2, 4, 16, 0, 0, 0, 0, 0, 0, 0, 2>;

fn setup_memory_core() -> (
    TestCore,
    AgentId,
    agent_kernel_core::ResourceId,
    agent_kernel_core::CapabilityId,
) {
    let mut core = TestCore::new();
    let agent = AgentId::new(1);
    core.register_agent(agent)
        .expect("agent registration should fit");
    let memory = core
        .register_resource(ResourceKind::Memory, None)
        .expect("memory resource should fit");
    let capability = core
        .grant_capability(
            agent,
            memory,
            OperationSet::empty()
                .with(Operation::Observe)
                .with(Operation::Act),
        )
        .expect("memory capability should fit");

    (core, agent, memory, capability)
}

#[test]
fn create_memory_cell_rejects_non_memory_resource_without_event() {
    let mut core = TestCore::new();
    let agent = AgentId::new(1);
    core.register_agent(agent)
        .expect("agent registration should fit");
    let workspace = core
        .register_resource(ResourceKind::Workspace, None)
        .expect("workspace resource should fit");
    let capability = core
        .grant_capability(agent, workspace, OperationSet::only(Operation::Act))
        .expect("workspace capability should fit");
    let events_before = core.events().len();

    assert_eq!(
        core.create_memory_cell(agent, capability, workspace, MemoryValue::new([1, 2, 3, 4])),
        Err(KernelError::ResourceKindMismatch)
    );
    assert!(core.memory_cells().is_empty());
    assert_eq!(core.events().len(), events_before);
}

#[test]
fn remember_memory_cell_requires_act_authority_without_mutation() {
    let (mut core, agent, memory, capability) = setup_memory_core();
    let cell = core
        .create_memory_cell(agent, capability, memory, MemoryValue::new([1, 1, 1, 1]))
        .expect("memory cell should fit");
    let observe_only = core
        .grant_capability(agent, memory, OperationSet::only(Operation::Observe))
        .expect("observe capability should fit");
    let events_before = core.events().len();

    assert_eq!(
        core.remember_memory_cell(agent, observe_only, cell, MemoryValue::new([2, 2, 2, 2])),
        Err(KernelError::OperationDenied)
    );
    assert_eq!(core.memory_cells()[0].value, MemoryValue::new([1, 1, 1, 1]));
    assert_eq!(core.memory_cells()[0].revision, 1);
    assert_eq!(core.events().len(), events_before);
}

#[test]
fn recall_memory_cell_requires_observe_authority_without_event() {
    let (mut core, agent, memory, capability) = setup_memory_core();
    let cell = core
        .create_memory_cell(agent, capability, memory, MemoryValue::new([3, 3, 3, 3]))
        .expect("memory cell should fit");
    let act_only = core
        .grant_capability(agent, memory, OperationSet::only(Operation::Act))
        .expect("act capability should fit");
    let events_before = core.events().len();

    assert_eq!(
        core.recall_memory_cell(agent, act_only, cell),
        Err(KernelError::OperationDenied)
    );
    assert_eq!(core.events().len(), events_before);
}

#[test]
fn suspended_actor_is_rejected_before_memory_cell_lookup() {
    let (mut core, agent, _, _) = setup_memory_core();
    core.suspend_agent(agent).expect("agent should suspend");
    let events_before = core.events().len();

    assert_eq!(
        core.recall_memory_cell(
            agent,
            agent_kernel_core::CapabilityId::new(99),
            MemoryCellId::new(99)
        ),
        Err(KernelError::AgentSuspended)
    );
    assert_eq!(core.events().len(), events_before);
}

#[test]
fn create_memory_cell_store_full_leaves_events_unchanged() {
    let mut core = KernelCore::<1, 1, 1, 8, 0, 0, 0, 0, 0, 0, 0, 1>::new();
    let agent = AgentId::new(1);
    core.register_agent(agent).expect("agent should fit");
    let memory = core
        .register_resource(ResourceKind::Memory, None)
        .expect("memory resource should fit");
    let capability = core
        .grant_capability(agent, memory, OperationSet::only(Operation::Act))
        .expect("capability should fit");
    core.create_memory_cell(agent, capability, memory, MemoryValue::new([1, 0, 0, 0]))
        .expect("first cell should fit");
    let events_before = core.events().len();

    assert_eq!(
        core.create_memory_cell(agent, capability, memory, MemoryValue::new([2, 0, 0, 0])),
        Err(KernelError::MemoryCellStoreFull)
    );
    assert_eq!(core.memory_cells().len(), 1);
    assert_eq!(core.events().len(), events_before);
}

#[test]
fn create_memory_cell_event_log_full_leaves_cells_unchanged() {
    let mut core = KernelCore::<1, 1, 1, 2, 0, 0, 0, 0, 0, 0, 0, 1>::new();
    let agent = AgentId::new(1);
    core.register_agent(agent)
        .expect("registration should consume one event");
    let memory = core
        .register_resource(ResourceKind::Memory, None)
        .expect("memory resource should fit");
    let capability = core
        .grant_capability(agent, memory, OperationSet::only(Operation::Act))
        .expect("grant should consume final event");

    assert_eq!(
        core.create_memory_cell(agent, capability, memory, MemoryValue::new([1, 0, 0, 0])),
        Err(KernelError::EventLogFull)
    );
    assert!(core.memory_cells().is_empty());
    assert_eq!(core.events().len(), 2);
}

#[test]
fn remember_memory_cell_event_log_full_leaves_value_unchanged() {
    let mut core = KernelCore::<1, 1, 1, 3, 0, 0, 0, 0, 0, 0, 0, 1>::new();
    let agent = AgentId::new(1);
    core.register_agent(agent).expect("agent should fit");
    let memory = core
        .register_resource(ResourceKind::Memory, None)
        .expect("memory resource should fit");
    let capability = core
        .grant_capability(
            agent,
            memory,
            OperationSet::empty()
                .with(Operation::Observe)
                .with(Operation::Act),
        )
        .expect("capability should fit");
    let cell = core
        .create_memory_cell(agent, capability, memory, MemoryValue::new([1, 0, 0, 0]))
        .expect("create should consume final event");

    assert_eq!(
        core.remember_memory_cell(agent, capability, cell, MemoryValue::new([2, 0, 0, 0])),
        Err(KernelError::EventLogFull)
    );
    assert_eq!(core.memory_cells()[0].value, MemoryValue::new([1, 0, 0, 0]));
    assert_eq!(core.memory_cells()[0].revision, 1);
    assert_eq!(core.events().len(), 3);
}

#[test]
fn recall_memory_cell_event_log_full_does_not_return_unaudited_value() {
    let mut core = KernelCore::<1, 1, 1, 3, 0, 0, 0, 0, 0, 0, 0, 1>::new();
    let agent = AgentId::new(1);
    core.register_agent(agent).expect("agent should fit");
    let memory = core
        .register_resource(ResourceKind::Memory, None)
        .expect("memory resource should fit");
    let capability = core
        .grant_capability(
            agent,
            memory,
            OperationSet::empty()
                .with(Operation::Observe)
                .with(Operation::Act),
        )
        .expect("capability should fit");
    let cell = core
        .create_memory_cell(agent, capability, memory, MemoryValue::new([1, 0, 0, 0]))
        .expect("create should consume final event");

    assert_eq!(
        core.recall_memory_cell(agent, capability, cell),
        Err(KernelError::EventLogFull)
    );
    assert_eq!(core.memory_cells()[0].revision, 1);
    assert_eq!(core.events().len(), 3);
}
