//! Audited native handler for paired Agent Record and context retirement.
//!
//! The adapter rejects architecture-owned runtime liveness, invokes only the
//! public facade, and validates exact semantic evidence before replying to the
//! authenticated ring-3 caller.

use agent_kernel_core::{
    AgentExecutionState, AgentId, AgentStatus, CapabilityId, EventKind, Operation,
};

use super::super::state;
use crate::{
    agent_cpu::{PendingAgentCallCpu, ResumableAgentCpu},
    native_agent_runtime::NativeAgentRuntime,
    serial_write_line, X86BootedKernel,
};

pub(super) fn retire(
    booted: &mut X86BootedKernel,
    runtime: &NativeAgentRuntime,
    pending: PendingAgentCallCpu,
    authority: CapabilityId,
    target: AgentId,
) -> Option<ResumableAgentCpu> {
    pending.authenticated_request()?;
    if runtime.contains(target) {
        return None;
    }

    let context = pending.context();
    let target_record = booted
        .kernel()
        .agents()
        .iter()
        .find(|record| record.id == target)
        .copied()?;
    let target_context = booted.kernel().execution_context(target).ok()?;
    let management_resource = target_record.management_resource?;
    let event_start = booted.kernel().events().len();
    let capacity = booted.kernel().agent_capacity();
    let count = booted.kernel().agent_count();
    let retired_floor = booted.kernel().retired_agent_floor();
    let expected_floor = if target.raw() > retired_floor.raw() {
        target
    } else {
        retired_floor
    };

    let receipt = booted
        .kernel_mut()
        .sys_retire_agent_record(context.agent(), authority, target)
        .ok()?;
    let kernel = booted.kernel();
    let event = kernel.events().get(event_start)?;
    let paired = kernel
        .agents()
        .iter()
        .zip(kernel.execution_contexts())
        .all(|(record, context)| record.id == context.agent);
    if receipt.record() != target_record
        || receipt.context() != target_context
        || receipt.agent() != target
        || receipt.actor() != context.agent()
        || receipt.authority() != authority
        || receipt.management_resource() != management_resource
        || receipt.retired_floor() != expected_floor
        || target_record.status != AgentStatus::Retired
        || target_context.state != AgentExecutionState::Idle
        || target_context.task.is_some()
        || target_context.driver_invocation.is_some()
        || kernel.events().len() != event_start + 1
        || kernel.agent_capacity() != capacity
        || kernel.agent_count() + 1 != count
        || kernel.agents().iter().any(|record| record.id == target)
        || kernel
            .execution_contexts()
            .iter()
            .any(|record| record.agent == target)
        || !paired
        || kernel.retired_agent_floor() != expected_floor
        || event.kind != EventKind::AgentRecordRetired
        || event.agent != context.agent()
        || event.target_agent != Some(target)
        || event.resource != Some(management_resource)
        || event.capability != Some(authority)
        || event.operation != Some(Operation::Delegate)
        || !state::running(booted, context)
    {
        return None;
    }

    serial_write_line("AGENT_KERNEL_AGENT_CALL_AGENT_RECORD_RETIREMENT_OK");
    pending.acknowledge_agent_record_retirement(receipt)
}
