mod memory_cell_record_retirement_support;

use agent_kernel_core::{
    AgentEntryKind, KernelError, MemoryValue, NamespaceKey, NamespaceObject, Operation,
    OperationSet,
};

use memory_cell_record_retirement_support::{retire_backing_resource, setup};

#[test]
fn active_backing_resource_rejects_retirement_without_mutation() {
    let (mut core, fixture) = setup::<32>(AgentEntryKind::Supervisor);
    let cells = core.memory_cells().to_vec();
    let events = core.events().len();

    assert_eq!(
        core.retire_memory_cell_record(fixture.actor, fixture.authority, fixture.cell),
        Err(KernelError::MemoryCellRecordRetirementNotReady)
    );
    assert_eq!(core.memory_cells(), cells.as_slice());
    assert_eq!(core.events().len(), events);
}

#[test]
fn retirement_requires_a_launched_supervisor() {
    let (mut core, fixture) = setup::<32>(AgentEntryKind::Worker);
    retire_backing_resource(&mut core, fixture);

    assert_eq!(
        core.retire_memory_cell_record(fixture.actor, fixture.authority, fixture.cell),
        Err(KernelError::AgentEntryKindMismatch)
    );
}

#[test]
fn namespace_reference_blocks_retirement_atomically() {
    let (mut core, fixture) = setup::<40>(AgentEntryKind::Supervisor);
    core.bind_namespace_entry(
        fixture.actor,
        fixture.authority,
        fixture.root,
        NamespaceKey::new(7),
        NamespaceObject::MemoryCell(fixture.cell),
    )
    .unwrap();
    retire_backing_resource(&mut core, fixture);
    let cells = core.memory_cells().to_vec();
    let events = core.events().len();

    assert_eq!(
        core.retire_memory_cell_record(fixture.actor, fixture.authority, fixture.cell),
        Err(KernelError::MemoryCellRecordRetirementReferenced)
    );
    assert_eq!(core.memory_cells(), cells.as_slice());
    assert_eq!(core.events().len(), events);
}

#[test]
fn cleanup_authority_must_allow_rollback() {
    let (mut core, fixture) = setup::<40>(AgentEntryKind::Supervisor);
    let observe = core
        .grant_capability(
            fixture.actor,
            fixture.root,
            OperationSet::only(Operation::Observe),
        )
        .unwrap();
    retire_backing_resource(&mut core, fixture);

    assert_eq!(
        core.retire_memory_cell_record(fixture.actor, observe, fixture.cell),
        Err(KernelError::OperationDenied)
    );
}

#[test]
fn event_exhaustion_preserves_the_memory_cell_store() {
    let (mut core, fixture) = setup::<10>(AgentEntryKind::Supervisor);
    core.remember_memory_cell(
        fixture.actor,
        fixture.target.capability,
        fixture.cell,
        MemoryValue::new([1, 2, 3, 4]),
    )
    .unwrap();
    retire_backing_resource(&mut core, fixture);
    assert_eq!(core.events().len(), 10);
    let cells = core.memory_cells().to_vec();

    assert_eq!(
        core.retire_memory_cell_record(fixture.actor, fixture.authority, fixture.cell),
        Err(KernelError::EventLogFull)
    );
    assert_eq!(core.memory_cells(), cells.as_slice());
    assert_eq!(core.events().len(), 10);
}
