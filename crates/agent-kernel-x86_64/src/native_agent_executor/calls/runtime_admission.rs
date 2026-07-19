//! Audited Supervisor handlers for native Runtime Admission lifecycle calls.

use agent_kernel_core::{
    AgentId, CapabilityId, EventKind, Operation, RuntimeAdmissionId, RuntimeAdmissionStatus, TaskId,
};

use super::super::state;
use crate::{
    agent_cpu::{PendingAgentCallCpu, ResumableAgentCpu},
    serial_write_line, X86BootedKernel,
};

pub(super) fn request(
    booted: &mut X86BootedKernel,
    pending: PendingAgentCallCpu,
    authority: CapabilityId,
    target: AgentId,
    target_task: TaskId,
) -> Option<ResumableAgentCpu> {
    pending.authenticated_request()?;
    let context = pending.context();
    let event_start = booted.kernel().events().len();
    let queue_len = booted.kernel().run_queue().len();
    let admission = booted
        .kernel_mut()
        .sys_request_runtime_admission(context.agent(), authority, target, target_task)
        .ok()?;
    let kernel = booted.kernel();
    let record = kernel.runtime_admission(admission).ok()?;
    let event = kernel.events().get(event_start)?;
    if kernel.events().len() != event_start + 1
        || kernel.run_queue().len() != queue_len
        || event.kind != EventKind::RuntimeAdmissionRequested
        || event.agent != context.agent()
        || event.capability != Some(authority)
        || event.operation != Some(Operation::Delegate)
        || event.runtime_admission != Some(admission)
        || event.target_agent != Some(target)
        || event.task != Some(target_task)
        || event.agent_image != Some(record.image)
        || record.requester != context.agent()
        || record.authority != authority
        || record.target != target
        || record.task != target_task
        || record.status != RuntimeAdmissionStatus::Requested
        || record.failure.is_some()
        || !state::running(booted, context)
    {
        return None;
    }
    serial_write_line("AGENT_KERNEL_AGENT_CALL_RUNTIME_ADMISSION_REQUEST_OK");
    pending.acknowledge_runtime_admission(admission, target, target_task)
}

pub(super) fn compact(
    booted: &mut X86BootedKernel,
    pending: PendingAgentCallCpu,
    authority: CapabilityId,
    through: RuntimeAdmissionId,
) -> Option<ResumableAgentCpu> {
    pending.authenticated_request()?;
    let context = pending.context();
    let event_start = booted.kernel().events().len();
    let queue_len = booted.kernel().run_queue().len();
    let admission_len = booted.kernel().runtime_admissions().len();
    let receipt = booted
        .kernel_mut()
        .sys_compact_runtime_admission_prefix(context.agent(), authority, through)
        .ok()?;
    let kernel = booted.kernel();
    let events = kernel.events().get(event_start..)?;
    if receipt.through() != through
        || receipt.count() == 0
        || receipt.count() > admission_len
        || kernel.runtime_admissions().len() + receipt.count() != admission_len
        || kernel.run_queue().len() != queue_len
        || events.len() != receipt.count()
        || events.first()?.runtime_admission != Some(receipt.first())
        || events.last()?.runtime_admission != Some(receipt.through())
        || events.iter().enumerate().any(|(index, event)| {
            event.sequence != (event_start + index + 1) as u64
                || event.kind != EventKind::RuntimeAdmissionCompacted
                || event.agent != context.agent()
                || event.capability != Some(authority)
                || event.operation != Some(Operation::Delegate)
                || event.resource.is_none()
                || event.task.is_none()
                || event.target_agent.is_none()
                || event.agent_image.is_none()
                || event.runtime_admission.is_none()
                || kernel
                    .runtime_admissions()
                    .iter()
                    .any(|record| Some(record.id) == event.runtime_admission)
        })
        || !state::running(booted, context)
    {
        return None;
    }
    serial_write_line("AGENT_KERNEL_AGENT_CALL_RUNTIME_ADMISSION_COMPACTION_OK");
    pending.acknowledge_runtime_admission_compaction(receipt)
}
