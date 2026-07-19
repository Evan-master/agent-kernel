mod memory_cell_record_retirement_support;

use agent_kernel_core::{
    AgentEntryKind, EventKind, MemoryCellId, MemoryValue, Operation, ResourceKind,
};

use memory_cell_record_retirement_support::{memory_operations, retire_backing_resource, setup};

#[test]
fn retirement_preserves_dense_order_and_reuses_capacity_with_fresh_ids() {
    let (mut core, fixture) = setup::<64>(AgentEntryKind::Supervisor);
    let trailing_resource = core
        .create_resource(
            fixture.actor,
            ResourceKind::Memory,
            Some((fixture.root, fixture.authority)),
            memory_operations(),
        )
        .unwrap();
    let trailing_cell = core
        .create_memory_cell(
            fixture.actor,
            trailing_resource.capability,
            trailing_resource.resource,
            MemoryValue::new([55, 66, 77, 88]),
        )
        .unwrap();
    retire_backing_resource(&mut core, fixture);
    let record = core.memory_cells()[0];
    let event_start = core.events().len();

    let receipt = core
        .retire_memory_cell_record(fixture.actor, fixture.authority, fixture.cell)
        .expect("terminal unreferenced MemoryCell retires");

    assert_eq!(receipt.record(), record);
    assert_eq!(receipt.memory_cell(), fixture.cell);
    assert_eq!(receipt.actor(), fixture.actor);
    assert_eq!(receipt.authority(), fixture.authority);
    assert_eq!(core.memory_cells().len(), 1);
    assert_eq!(core.memory_cells()[0].id, trailing_cell);

    let event = core.events()[event_start];
    assert_eq!(event.kind, EventKind::MemoryCellRecordRetired);
    assert_eq!(event.agent, fixture.actor);
    assert_eq!(event.resource, Some(fixture.target.resource));
    assert_eq!(event.capability, Some(fixture.authority));
    assert_eq!(event.memory_cell, Some(fixture.cell));
    assert_eq!(event.operation, Some(Operation::Rollback));
    assert_eq!(event.target_agent, Some(fixture.actor));

    let fresh = core
        .create_memory_cell(
            fixture.actor,
            trailing_resource.capability,
            trailing_resource.resource,
            MemoryValue::new([99, 100, 101, 102]),
        )
        .expect("returned dense slot is reusable");
    assert_eq!(fresh, MemoryCellId::new(3));
    assert!(fresh.raw() > fixture.cell.raw());
    assert_eq!(core.memory_cells().len(), 2);
    assert_eq!(core.memory_cells()[1].id, fresh);
}

#[test]
fn complete_record_is_preserved_in_the_receipt() {
    let (mut core, fixture) = setup::<32>(AgentEntryKind::Supervisor);
    core.remember_memory_cell(
        fixture.actor,
        fixture.target.capability,
        fixture.cell,
        MemoryValue::new([101, 202, 303, 404]),
    )
    .unwrap();
    retire_backing_resource(&mut core, fixture);

    let receipt = core
        .retire_memory_cell_record(fixture.actor, fixture.authority, fixture.cell)
        .unwrap();

    assert_eq!(receipt.record().creator, fixture.actor);
    assert_eq!(receipt.record().last_writer, fixture.actor);
    assert_eq!(
        receipt.record().value,
        MemoryValue::new([101, 202, 303, 404])
    );
    assert_eq!(receipt.record().revision, 2);
}
