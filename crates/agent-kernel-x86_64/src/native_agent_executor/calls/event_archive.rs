//! Audited Supervisor handler for native Event archive handoff and commit.
//!
//! The handler snapshots the complete bounded live log, preflights the external
//! architecture archive, invokes the public two-phase facade, then validates
//! checkpoint identity, dense removal, suffix stability, and sequence state.

use agent_kernel_core::{CapabilityId, Event};

use super::super::{state, NativeExecutionReport};
use crate::{
    agent_cpu::{PendingAgentCallCpu, ResumableAgentCpu},
    serial_write_line, X86BootedKernel, X86_EVENT_CAPACITY,
};

pub(super) fn archive(
    booted: &mut X86BootedKernel,
    report: &mut NativeExecutionReport,
    pending: PendingAgentCallCpu,
    authority: CapabilityId,
    through_sequence: u64,
) -> Option<ResumableAgentCpu> {
    pending.authenticated_request()?;
    let context = pending.context();
    let proposal = booted
        .kernel()
        .sys_prepare_event_archive(through_sequence)
        .ok()?;
    let event_len = booted.kernel().events().len();
    let next_sequence = booted.kernel().next_event_sequence();
    let task_len = booted.kernel().tasks().len();
    let queue_len = booted.kernel().run_queue().len();
    if event_len > X86_EVENT_CAPACITY
        || proposal.count() > event_len
        || !report.can_record_event_archive(proposal.count())
    {
        return None;
    }

    let mut previous: [Option<Event>; X86_EVENT_CAPACITY] = [None; X86_EVENT_CAPACITY];
    for (index, event) in booted.kernel().events().iter().copied().enumerate() {
        previous[index] = Some(event);
    }

    let checkpoint = booted
        .kernel_mut()
        .sys_commit_event_archive(context.agent(), authority, proposal)
        .ok()?;
    let kernel = booted.kernel();
    if checkpoint.proposal() != proposal
        || checkpoint.actor() != context.agent()
        || checkpoint.authority() != authority
        || kernel.event_archive_checkpoint() != Some(checkpoint)
        || kernel.events().len() + checkpoint.count() != event_len
        || kernel.next_event_sequence() != next_sequence
        || kernel.tasks().len() != task_len
        || kernel.run_queue().len() != queue_len
        || kernel.events().iter().enumerate().any(|(index, event)| {
            previous.get(index + checkpoint.count()).copied().flatten() != Some(*event)
        })
        || !state::running(booted, context)
    {
        return None;
    }
    report.record_event_archive(event_len, &previous[..checkpoint.count()], checkpoint)?;

    serial_write_line("AGENT_KERNEL_AGENT_CALL_EVENT_ARCHIVE_OK");
    pending.acknowledge_event_archive(checkpoint)
}
