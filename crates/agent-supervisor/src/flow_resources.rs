//! Resource-oriented supervisor flow after the task lifecycle completes.
//!
//! This supervisor-layer module keeps host-side demonstration code out of the
//! kernel crates. It drives only syscall-style facade methods, owns no kernel
//! internals directly, and exists to keep `main.rs` focused on the primary task
//! lifecycle flow.

use agent_kernel::AgentKernel;
use agent_kernel_core::{
    AgentId, CapabilityId, MemoryValue, NamespaceKey, NamespaceObject, Operation, OperationSet,
    ResourceId, ResourceKind, TaskId,
};

pub type SupervisorKernel = AgentKernel<8, 8, 8, 56, 8, 8, 8, 8, 8, 8, 8, 8, 8, 1, 1, 1, 1>;

pub struct ResourceFlowContext {
    pub agent: AgentId,
    pub target_agent: AgentId,
    pub owner_capability: CapabilityId,
    pub workspace: ResourceId,
    pub task: TaskId,
}

pub fn drive_resource_flow(kernel: &mut SupervisorKernel, context: ResourceFlowContext) {
    let memory = kernel
        .sys_register_resource(ResourceKind::Memory, None)
        .expect("memory resource should fit in simulator kernel");
    let memory_capability = kernel
        .sys_grant(
            context.agent,
            memory,
            OperationSet::empty()
                .with(Operation::Observe)
                .with(Operation::Act),
        )
        .expect("memory capability should fit in simulator kernel");
    let memory_cell = kernel
        .sys_create_memory_cell(
            context.agent,
            memory_capability,
            memory,
            MemoryValue::new([1, 2, 3, 4]),
        )
        .expect("memory cell should fit in simulator kernel");
    let recalled = kernel
        .sys_recall_memory_cell(context.agent, memory_capability, memory_cell)
        .expect("agent should recall memory cell");
    assert_eq!(recalled, MemoryValue::new([1, 2, 3, 4]));
    kernel
        .sys_remember_memory_cell(
            context.agent,
            memory_capability,
            memory_cell,
            MemoryValue::new([4, 3, 2, 1]),
        )
        .expect("agent should remember new memory cell value");

    let namespace_key = NamespaceKey::new(1);
    let namespace_entry = kernel
        .sys_bind_namespace_entry(
            context.agent,
            context.owner_capability,
            context.workspace,
            namespace_key,
            NamespaceObject::MemoryCell(memory_cell),
        )
        .expect("agent should bind memory cell in workspace namespace");
    let resolved = kernel
        .sys_resolve_namespace_entry(
            context.agent,
            context.owner_capability,
            context.workspace,
            namespace_key,
        )
        .expect("agent should resolve workspace namespace entry");
    assert_eq!(resolved, NamespaceObject::MemoryCell(memory_cell));
    kernel
        .sys_rebind_namespace_entry(
            context.agent,
            context.owner_capability,
            namespace_entry,
            NamespaceObject::Task(context.task),
        )
        .expect("agent should rebind namespace entry to task");

    let service = kernel
        .sys_create_resource(
            context.agent,
            ResourceKind::Service,
            Some((context.workspace, context.owner_capability)),
            OperationSet::only(Operation::Rollback),
        )
        .expect("owned service resource should fit in simulator kernel");
    kernel
        .sys_retire_resource(context.agent, service.capability, service.resource)
        .expect("agent should retire service resource");
    let target_observe_capability = kernel
        .sys_derive_capability(
            context.agent,
            context.owner_capability,
            context.target_agent,
            OperationSet::only(Operation::Observe),
        )
        .expect("owner should derive observe authority to target agent");
    kernel
        .sys_observe(
            context.target_agent,
            target_observe_capability,
            context.workspace,
        )
        .expect("target agent should observe through derived capability");
}
