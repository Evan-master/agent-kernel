//! Audited Supervisor handler for native Intent Store compaction.

use agent_kernel_core::{CapabilityId, EventKind, IntentId, Operation};

use super::super::state;
use crate::{
    agent_cpu::{PendingAgentCallCpu, ResumableAgentCpu},
    serial_write_line, X86BootedKernel,
};

pub(super) fn compact(
    booted: &mut X86BootedKernel,
    pending: PendingAgentCallCpu,
    authority: CapabilityId,
    through: IntentId,
) -> Option<ResumableAgentCpu> {
    pending.authenticated_request()?;
    let context = pending.context();
    let event_start = booted.kernel().events().len();
    let queue_len = booted.kernel().run_queue().len();
    let task_len = booted.kernel().tasks().len();
    let intent_len = booted.kernel().intents().len();
    let receipt = booted
        .kernel_mut()
        .sys_compact_intent_prefix(context.agent(), authority, through)
        .ok()?;
    let kernel = booted.kernel();
    let events = kernel.events().get(event_start..)?;
    if receipt.through() != through
        || receipt.count() == 0
        || receipt.count() > intent_len
        || kernel.intents().len() + receipt.count() != intent_len
        || kernel.tasks().len() != task_len
        || kernel.run_queue().len() != queue_len
        || events.len() != receipt.count()
        || events.first()?.intent != Some(receipt.first())
        || events.last()?.intent != Some(receipt.through())
        || events.iter().enumerate().any(|(index, event)| {
            event.sequence != (event_start + index + 1) as u64
                || event.kind != EventKind::IntentCompacted
                || event.agent != context.agent()
                || event.capability != Some(authority)
                || event.operation != Some(Operation::Rollback)
                || event.resource.is_none()
                || event.intent.is_none()
                || event.intent_kind.is_none()
                || event.target_agent.is_none()
                || kernel
                    .intents()
                    .iter()
                    .any(|intent| Some(intent.id) == event.intent)
        })
        || kernel
            .tasks()
            .iter()
            .any(|task| kernel.intent(task.intent).is_err())
        || !state::running(booted, context)
    {
        return None;
    }
    serial_write_line("AGENT_KERNEL_AGENT_CALL_INTENT_COMPACTION_OK");
    pending.acknowledge_intent_compaction(receipt)
}
