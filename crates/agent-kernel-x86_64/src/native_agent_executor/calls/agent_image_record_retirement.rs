//! Audited native handler for terminal Agent Image record retirement.
//!
//! The adapter rejects every architecture-owned context carrying the target
//! Image, invokes the public facade, and validates exact semantic evidence
//! before replying to the authenticated ring-3 caller.

use agent_kernel_core::{AgentImageId, AgentImageStatus, CapabilityId, EventKind, Operation};

use super::super::{state, NativeExecutionReport};
use crate::{
    agent_cpu::{PendingAgentCallCpu, ResumableAgentCpu},
    native_agent_runtime::NativeAgentRuntime,
    serial_write_line, X86BootedKernel,
};

pub(super) fn retire(
    booted: &mut X86BootedKernel,
    runtime: &NativeAgentRuntime,
    report: &NativeExecutionReport,
    pending: PendingAgentCallCpu,
    authority: CapabilityId,
    target: AgentImageId,
) -> Option<ResumableAgentCpu> {
    pending.authenticated_request()?;
    let context = pending.context();
    if context.image() == target || runtime.contains_image(target) || report.contains_image(target)
    {
        return None;
    }

    let target_record = booted.kernel().agent_image(target).ok()?;
    let event_start = booted.kernel().events().len();
    let count = booted.kernel().agent_images().len();
    let receipt = booted
        .kernel_mut()
        .sys_retire_agent_image_record(context.agent(), authority, target)
        .ok()?;
    let kernel = booted.kernel();
    let event = kernel.events().get(event_start)?;
    if receipt.record() != target_record
        || receipt.image() != target
        || receipt.actor() != context.agent()
        || receipt.authority() != authority
        || target_record.status != AgentImageStatus::Retired
        || kernel.events().len() != event_start + 1
        || kernel.agent_images().len() + 1 != count
        || kernel
            .agent_images()
            .iter()
            .any(|record| record.id == target)
        || event.kind != EventKind::AgentImageRecordRetired
        || event.agent != context.agent()
        || event.target_agent != Some(target_record.owner)
        || event.resource != Some(target_record.resource)
        || event.capability != Some(authority)
        || event.operation != Some(Operation::Rollback)
        || event.agent_image != Some(target)
        || event.agent_image_kind != Some(target_record.kind)
        || event.agent_image_digest != Some(target_record.digest)
        || event.agent_image_abi_version != Some(target_record.abi_version)
        || event.agent_image_entry_version != Some(target_record.entry_version)
        || !state::running(booted, context)
    {
        return None;
    }

    serial_write_line("AGENT_KERNEL_AGENT_CALL_AGENT_IMAGE_RECORD_RETIREMENT_OK");
    pending.acknowledge_agent_image_record_retirement(receipt)
}
