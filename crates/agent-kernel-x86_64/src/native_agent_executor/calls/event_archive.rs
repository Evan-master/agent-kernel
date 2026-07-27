//! Audited Supervisor handler for native Event archive handoff and commit.
//!
//! The handler snapshots the complete bounded live log, preflights the external
//! architecture archive, invokes the public two-phase facade, then validates
//! checkpoint identity, dense removal, suffix stability, and sequence state.

use agent_kernel_core::{CapabilityId, Event, KernelError};

use super::super::{state, NativeExecutionReport};
use crate::{
    agent_cpu::{PendingAgentCallCpu, ResumableAgentCpu},
    serial_write_line, serial_write_str, serial_write_u64, X86BootedKernel, X86_EVENT_CAPACITY,
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
    let snapshot = booted
        .kernel()
        .sys_prepare_event_archive_snapshot(context.agent(), authority, through_sequence)
        .ok()?;
    let proposal = snapshot.proposal();
    let event_len = booted.kernel().events().len();
    let next_sequence = booted.kernel().next_event_sequence();
    let task_len = booted.kernel().tasks().len();
    let queue_len = booted.kernel().run_queue().len();
    let previous_checkpoint = booted.kernel().event_archive_checkpoint();
    let previous_receipt = booted.kernel().durable_archive_receipt();
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

    let commit = booted
        .kernel_mut()
        .sys_commit_event_archive(context.agent(), authority, proposal);
    match commit {
        Ok(checkpoint) => {
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
            write_digest("AGENT_KERNEL_EVENT_ARCHIVE_DIGEST_", checkpoint.digest());
            serial_write_line("AGENT_KERNEL_AGENT_CALL_EVENT_ARCHIVE_OK");
            pending.acknowledge_event_archive(checkpoint)
        }
        Err(KernelError::EventArchiveDurabilityRequired) => {
            let kernel = booted.kernel();
            if kernel.events().len() != event_len
                || kernel.next_event_sequence() != next_sequence
                || kernel.tasks().len() != task_len
                || kernel.run_queue().len() != queue_len
                || kernel.event_archive_checkpoint() != previous_checkpoint
                || kernel.durable_archive_receipt() != previous_receipt
                || kernel
                    .events()
                    .iter()
                    .enumerate()
                    .any(|(index, event)| previous.get(index).copied().flatten() != Some(*event))
                || !state::running(booted, context)
            {
                return None;
            }
            report.record_event_snapshot(event_len, &previous[..proposal.count()], snapshot)?;
            write_digest("AGENT_KERNEL_EVENT_SNAPSHOT_DIGEST_", proposal.digest());
            serial_write_line("AGENT_KERNEL_AGENT_CALL_EVENT_SNAPSHOT_OK");
            pending.acknowledge_event_archive_snapshot(proposal)
        }
        Err(_) => {
            serial_write_line("AGENT_KERNEL_EVENT_ARCHIVE_COMMIT_ERROR");
            None
        }
    }
}

fn write_digest(prefix: &str, digest: agent_kernel_core::EventArchiveDigest) {
    for (index, word) in digest.words_le().iter().copied().enumerate() {
        serial_write_str(prefix);
        serial_write_u64(index as u64);
        serial_write_str("=");
        serial_write_u64(word);
        serial_write_line("");
    }
}
