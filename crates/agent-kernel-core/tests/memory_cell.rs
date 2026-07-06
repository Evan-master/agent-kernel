use agent_kernel_core::{
    AgentId, EventKind, KernelCore, MemoryCellId, MemoryValue, Operation, OperationSet, ResourceId,
    ResourceKind,
};

type TestCore = KernelCore<2, 2, 2, 16, 0, 0, 0, 0, 0, 0, 0, 2>;

fn setup_memory_core() -> (
    TestCore,
    AgentId,
    ResourceId,
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
fn create_memory_cell_records_value_and_event() {
    let (mut core, agent, memory, capability) = setup_memory_core();
    let value = MemoryValue::new([1, 2, 3, 4]);

    let cell = core
        .create_memory_cell(agent, capability, memory, value)
        .expect("memory cell should fit");

    assert_eq!(cell, MemoryCellId::new(1));
    assert_eq!(core.memory_cells().len(), 1);
    assert_eq!(core.memory_cells()[0].id, cell);
    assert_eq!(core.memory_cells()[0].resource, memory);
    assert_eq!(core.memory_cells()[0].creator, agent);
    assert_eq!(core.memory_cells()[0].last_writer, agent);
    assert_eq!(core.memory_cells()[0].value, value);
    assert_eq!(core.memory_cells()[0].revision, 1);
    assert_eq!(core.events()[2].kind, EventKind::MemoryCellCreated);
    assert_eq!(core.events()[2].agent, agent);
    assert_eq!(core.events()[2].resource, Some(memory));
    assert_eq!(core.events()[2].capability, Some(capability));
    assert_eq!(core.events()[2].memory_cell, Some(cell));
    assert_eq!(core.events()[2].operation, Some(Operation::Act));
}

#[test]
fn recall_memory_cell_returns_value_and_records_audit_event() {
    let (mut core, agent, memory, capability) = setup_memory_core();
    let value = MemoryValue::new([5, 6, 7, 8]);
    let cell = core
        .create_memory_cell(agent, capability, memory, value)
        .expect("memory cell should fit");

    let recalled = core
        .recall_memory_cell(agent, capability, cell)
        .expect("agent should recall memory cell");

    assert_eq!(recalled, value);
    assert_eq!(core.events()[3].kind, EventKind::MemoryCellRecalled);
    assert_eq!(core.events()[3].agent, agent);
    assert_eq!(core.events()[3].memory_cell, Some(cell));
    assert_eq!(core.events()[3].operation, Some(Operation::Observe));
}

#[test]
fn remember_memory_cell_updates_value_revision_and_event() {
    let (mut core, agent, memory, capability) = setup_memory_core();
    let cell = core
        .create_memory_cell(agent, capability, memory, MemoryValue::new([1, 1, 1, 1]))
        .expect("memory cell should fit");
    let new_value = MemoryValue::new([9, 8, 7, 6]);

    let event = core
        .remember_memory_cell(agent, capability, cell, new_value)
        .expect("agent should remember new value");

    assert_eq!(core.memory_cells()[0].value, new_value);
    assert_eq!(core.memory_cells()[0].revision, 2);
    assert_eq!(core.memory_cells()[0].last_writer, agent);
    assert_eq!(event.kind, EventKind::MemoryCellRemembered);
    assert_eq!(event.agent, agent);
    assert_eq!(event.resource, Some(memory));
    assert_eq!(event.memory_cell, Some(cell));
    assert_eq!(event.operation, Some(Operation::Act));
}
