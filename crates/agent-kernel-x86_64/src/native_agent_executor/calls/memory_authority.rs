//! Shared semantic authority lookup for native runtime-memory calls.
//!
//! This bare-metal executor child authenticates the running call context and
//! reads facade-exposed Memory Resource, Capability, and MemoryCell records.
//! Physical ownership remains in the operation handlers.

use agent_kernel_core::{
    AgentId, CapabilityId, MemoryCellId, MemoryCellRecord, Operation, ResourceId, ResourceKind,
    ResourceStatus,
};

use crate::{agent_cpu::PendingAgentCallCpu, X86BootedKernel};

pub(super) fn cell(booted: &X86BootedKernel, cell: MemoryCellId) -> Option<MemoryCellRecord> {
    booted
        .kernel()
        .memory_cells()
        .iter()
        .find(|record| record.id == cell)
        .copied()
}

pub(super) fn valid(
    booted: &X86BootedKernel,
    agent: AgentId,
    capability: CapabilityId,
    resource: ResourceId,
    operation: Operation,
) -> bool {
    let kernel = booted.kernel();
    matches!(kernel.resources().iter().find(|record| record.id == resource), Some(record)
        if record.kind == ResourceKind::Memory && record.status == ResourceStatus::Active)
        && matches!(kernel.capability(capability), Ok(record)
            if record.agent == agent
                && record.resource == resource
                && record.operations.allows(operation)
                && !record.revoked
                && record.task.is_none())
}

pub(super) fn authenticated_context(
    pending: &PendingAgentCallCpu,
) -> Option<agent_kernel_x86_64::agent_call::AgentCallContext> {
    pending.authenticated_request()?;
    Some(pending.context())
}
