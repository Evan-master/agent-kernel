use agent_kernel::AgentKernel;
use agent_kernel_core::{
    AgentId, EventKind, MemoryCellId, MemoryValue, Operation, OperationSet, ResourceKind,
};

type TestKernel = AgentKernel<1, 1, 1, 8, 0, 0, 0, 0, 0, 0, 0, 1>;

#[test]
fn memory_cell_syscalls_create_recall_remember_and_expose_cells() {
    let mut kernel = TestKernel::new();
    let agent = AgentId::new(1);
    kernel
        .sys_register_agent(agent)
        .expect("agent registration should fit");
    let memory = kernel
        .sys_register_resource(ResourceKind::Memory, None)
        .expect("memory resource should fit");
    let capability = kernel
        .sys_grant(
            agent,
            memory,
            OperationSet::empty()
                .with(Operation::Observe)
                .with(Operation::Act),
        )
        .expect("memory capability should fit");

    let cell = kernel
        .sys_create_memory_cell(agent, capability, memory, MemoryValue::new([1, 2, 3, 4]))
        .expect("memory cell should fit");
    let recalled = kernel
        .sys_recall_memory_cell(agent, capability, cell)
        .expect("memory cell should recall");
    let event = kernel
        .sys_remember_memory_cell(agent, capability, cell, MemoryValue::new([4, 3, 2, 1]))
        .expect("memory cell should remember new value");

    assert_eq!(cell, MemoryCellId::new(1));
    assert_eq!(recalled, MemoryValue::new([1, 2, 3, 4]));
    assert_eq!(
        kernel.memory_cells()[0].value,
        MemoryValue::new([4, 3, 2, 1])
    );
    assert_eq!(kernel.memory_cells()[0].revision, 2);
    assert_eq!(kernel.events()[2].kind, EventKind::MemoryCellCreated);
    assert_eq!(kernel.events()[3].kind, EventKind::MemoryCellRecalled);
    assert_eq!(event.kind, EventKind::MemoryCellRemembered);
}
