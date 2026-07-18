//! Audited Supervisor request handler for native runtime admission.

use agent_kernel_core::{
    AgentId, CapabilityId, EventKind, Operation, RuntimeAdmissionStatus, TaskId,
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
