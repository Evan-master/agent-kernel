//! Audited Supervisor handler for native Task Store compaction.

use agent_kernel_core::{CapabilityId, EventKind, Operation, TaskId};

use super::super::state;
use crate::{
    agent_cpu::{PendingAgentCallCpu, ResumableAgentCpu},
    serial_write_line, X86BootedKernel,
};

pub(super) fn compact(
    booted: &mut X86BootedKernel,
    pending: PendingAgentCallCpu,
    authority: CapabilityId,
    through: TaskId,
) -> Option<ResumableAgentCpu> {
    pending.authenticated_request()?;
    let context = pending.context();
    let event_start = booted.kernel().events().len();
    let queue_len = booted.kernel().run_queue().len();
    let task_len = booted.kernel().tasks().len();
    let receipt = booted
        .kernel_mut()
        .sys_compact_task_prefix(context.agent(), authority, through)
        .ok()?;
    let kernel = booted.kernel();
    let events = kernel.events().get(event_start..)?;
    if receipt.through() != through
        || receipt.count() == 0
        || receipt.count() > task_len
        || kernel.tasks().len() + receipt.count() != task_len
        || kernel.run_queue().len() != queue_len
        || events.len() != receipt.count()
        || events.first()?.task != Some(receipt.first())
        || events.last()?.task != Some(receipt.through())
        || events.iter().enumerate().any(|(index, event)| {
            event.sequence != (event_start + index + 1) as u64
                || event.kind != EventKind::TaskCompacted
                || event.agent != context.agent()
                || event.capability != Some(authority)
                || event.operation != Some(Operation::Rollback)
                || event.resource.is_none()
                || event.intent.is_none()
                || event.task.is_none()
                || kernel
                    .tasks()
                    .iter()
                    .any(|task| Some(task.id) == event.task)
        })
        || !state::running(booted, context)
    {
        return None;
    }
    serial_write_line("AGENT_KERNEL_AGENT_CALL_TASK_COMPACTION_OK");
    pending.acknowledge_task_compaction(receipt)
}
