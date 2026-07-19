use agent_kernel::AgentKernel;
use agent_kernel_core::{
    AgentEntryKind, AgentId, AgentImageDigest, AgentImageKind, EventKind, MemoryCellId,
    MemoryValue, Operation, OperationSet, ResourceKind,
};

type TestKernel = AgentKernel<2, 3, 5, 32, 0, 0, 0, 0, 0, 0, 0, 2, 0, 0, 0, 0, 0, 2, 0, 0, 0, 0, 0>;

#[test]
fn facade_retires_a_terminal_memory_cell_and_reuses_its_slot() {
    let mut kernel = TestKernel::new();
    let actor = AgentId::new(1);
    kernel.sys_register_agent(actor).unwrap();
    let root = kernel
        .sys_register_resource(ResourceKind::Workspace, None)
        .unwrap();
    let root_operations = OperationSet::only(Operation::Act)
        .with(Operation::Verify)
        .with(Operation::Rollback)
        .with(Operation::Delegate);
    let authority = kernel.sys_grant(actor, root, root_operations).unwrap();
    let image = kernel
        .sys_register_agent_image(
            actor,
            authority,
            root,
            AgentImageKind::Supervisor,
            AgentImageDigest::new([0x72; 32]),
            1,
            1,
        )
        .unwrap();
    kernel
        .sys_verify_agent_image(actor, authority, image)
        .unwrap();
    kernel
        .sys_launch_agent(
            actor,
            authority,
            root,
            image,
            AgentEntryKind::Supervisor,
            None,
        )
        .unwrap();
    let memory_operations = OperationSet::only(Operation::Observe)
        .with(Operation::Act)
        .with(Operation::Rollback);
    let target = kernel
        .sys_create_resource(
            actor,
            ResourceKind::Memory,
            Some((root, authority)),
            memory_operations,
        )
        .unwrap();
    let cell = kernel
        .sys_create_memory_cell(
            actor,
            target.capability,
            target.resource,
            MemoryValue::new([5, 6, 7, 8]),
        )
        .unwrap();
    kernel
        .sys_retire_resource(actor, target.capability, target.resource)
        .unwrap();

    let receipt = kernel
        .sys_retire_memory_cell_record(actor, authority, cell)
        .expect("facade retires the terminal MemoryCell record");

    assert_eq!(receipt.memory_cell(), cell);
    assert_eq!(receipt.record().resource, target.resource);
    assert_eq!(receipt.actor(), actor);
    assert_eq!(receipt.authority(), authority);
    assert_eq!(
        kernel.events().last().unwrap().kind,
        EventKind::MemoryCellRecordRetired
    );
    assert!(kernel.memory_cells().is_empty());

    let fresh = kernel.sys_create_memory_cell(
        actor,
        target.capability,
        target.resource,
        MemoryValue::new([9, 10, 11, 12]),
    );
    assert!(
        fresh.is_err(),
        "retired backing Resources reject active use"
    );

    let active = kernel
        .sys_create_resource(
            actor,
            ResourceKind::Memory,
            Some((root, authority)),
            memory_operations,
        )
        .unwrap();
    let fresh = kernel
        .sys_create_memory_cell(
            actor,
            active.capability,
            active.resource,
            MemoryValue::new([9, 10, 11, 12]),
        )
        .unwrap();
    assert_eq!(fresh, MemoryCellId::new(2));
}
