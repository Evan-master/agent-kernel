//! Audited Supervisor handler for native Agent Entry retirement.

use agent_kernel_core::{AgentId, CapabilityId, EventKind, KernelError, Operation};

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
    let event_start = booted.kernel().events().len();
    let task_len = booted.kernel().tasks().len();
    let admission_len = booted.kernel().runtime_admissions().len();
    let capability_count = booted.kernel().capability_count();
    let entry_capacity = booted.kernel().agent_entry_capacity();
    let entry_count = booted.kernel().agent_entry_count();
    let target_record = booted.kernel().agent_entry(target).ok()?;

    let receipt = booted
        .kernel_mut()
        .sys_retire_agent_entry(context.agent(), authority, target)
        .ok()?;
    let kernel = booted.kernel();
    let event = kernel.events().get(event_start)?;
    if receipt.agent() != target
        || receipt.entry() != target_record
        || kernel.events().len() != event_start + 1
        || kernel.agent_entry_capacity() != entry_capacity
        || kernel.agent_entry_count() + 1 != entry_count
        || kernel.agent_entry(target) != Err(KernelError::AgentEntryNotFound)
        || kernel.capability_count() != capability_count
        || kernel.capability(authority).is_err()
        || kernel.capability(target_record.capability).is_err()
        || kernel.tasks().len() != task_len
        || kernel.runtime_admissions().len() != admission_len
        || event.sequence != (event_start + 1) as u64
        || event.kind != EventKind::AgentEntryRetired
        || event.agent != context.agent()
        || event.target_agent != Some(target)
        || event.capability != Some(target_record.capability)
        || event.source_capability != Some(authority)
        || event.operation != Some(Operation::Rollback)
        || event.resource != Some(target_record.resource)
        || event.agent_image != Some(target_record.image)
        || event.agent_image_kind != Some(target_record.kind.image_kind())
        || event.intent != target_record.intent
        || event.task != target_record.task
        || !state::running(booted, context)
    {
        return None;
    }

    serial_write_line("AGENT_KERNEL_AGENT_CALL_AGENT_ENTRY_RETIREMENT_OK");
    pending.acknowledge_agent_entry_retirement(receipt)
}
