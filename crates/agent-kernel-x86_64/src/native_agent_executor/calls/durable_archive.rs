//! Native two-stage durable Event archive handler.
//!
//! Prepare performs read-only Core authorization and stages an unsigned
//! canonical request in the caller's private page. Commit snapshots that page,
//! verifies and persists the signed request, then consumes the resulting
//! machine proof for one Core release without returning to ring 3.

use agent_kernel_core::{
    AgentEntryKind, AgentImageKind, AgentImageStatus, CapabilityId, DurableSignatureAlgorithm,
    DurableStateSignerStatus, Event,
};
use agent_kernel_x86_64::{
    agent_call::AgentCallContext,
    ata::NativeDurableArchiveCaller,
    tpm2::{sign_retained_durable_request, KernelStateSigner},
};

use super::super::{state, NativeExecutionReport};
use crate::{
    agent_cpu::{PendingAgentCallCpu, ResumableAgentCpu},
    serial_write_line, serial_write_str, serial_write_u64, NativeDurableSession, X86BootedKernel,
    X86_EVENT_CAPACITY,
};

pub(super) fn prepare(
    booted: &mut X86BootedKernel,
    session: &mut NativeDurableSession<'_>,
    mut pending: PendingAgentCallCpu,
    archive_authority: CapabilityId,
    storage_authority: CapabilityId,
    through_sequence: u64,
    call_data_generation: u64,
) -> Option<ResumableAgentCpu> {
    pending.authenticated_request()?;
    let context = pending.context();
    let caller = caller(context)?;
    let proposal = booted
        .kernel()
        .sys_prepare_event_archive(through_sequence)
        .ok()?;
    let preflight = booted
        .kernel()
        .preflight_durable_event_archive(
            context.agent(),
            archive_authority,
            storage_authority,
            session.config().storage(),
            proposal,
        )
        .ok()?;
    let events = booted.kernel().events();
    if proposal.count() > events.len() || !state::running(booted, context) {
        return None;
    }
    let preparation = session
        .prepare(
            caller,
            preflight,
            &events[..proposal.count()],
            call_data_generation,
        )
        .ok()?;
    if !pending.stage_durable_archive_preparation(preparation) {
        session
            .cancel_preparation(caller, call_data_generation)
            .ok()?;
        return None;
    }

    let reply = pending.acknowledge_durable_archive_prepared(preparation);
    if reply.is_none() {
        session
            .cancel_preparation(caller, call_data_generation)
            .ok()?;
        return None;
    }
    serial_write_line("AGENT_KERNEL_DURABLE_ARCHIVE_PREPARED_OK");
    reply
}

pub(super) fn commit(
    booted: &mut X86BootedKernel,
    session: &mut NativeDurableSession<'_>,
    report: &mut NativeExecutionReport,
    pending: PendingAgentCallCpu,
    call_data_generation: u64,
) -> Option<ResumableAgentCpu> {
    let request_bytes = pending.authenticated_durable_archive_request()?;
    let context = pending.context();
    let caller = caller(context)?;
    let preparation = session.preparation()?;
    if preparation.caller() != caller
        || preparation.call_data_generation() != call_data_generation
        || !state::running(booted, context)
    {
        return None;
    }

    let prepared_preflight = preparation.preflight();
    let proposal = prepared_preflight.proposal();
    let preflight = booted
        .kernel()
        .preflight_durable_event_archive(
            context.agent(),
            prepared_preflight.archive_authority(),
            prepared_preflight.storage_authority(),
            session.config().storage(),
            proposal,
        )
        .ok()?;
    if preflight != prepared_preflight {
        return None;
    }

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

    let mut verified = session
        .commit_prepared(caller, preflight, &request_bytes)
        .ok()?;
    let receipt = verified.receipt();
    let checkpoint = booted
        .kernel_mut()
        .commit_verified_event_archive(
            context.agent(),
            preflight.archive_authority(),
            preflight.storage_authority(),
            proposal,
            receipt,
            &mut verified,
        )
        .ok()?;
    let kernel = booted.kernel();
    if !verified.is_consumed()
        || checkpoint.proposal() != proposal
        || checkpoint.actor() != context.agent()
        || checkpoint.authority() != preflight.archive_authority()
        || checkpoint.root() != preflight.root()
        || kernel.event_archive_checkpoint() != Some(checkpoint)
        || kernel.durable_archive_receipt() != Some(receipt)
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

    let digest = checkpoint.digest().words_le();
    write_digest_word("AGENT_KERNEL_DURABLE_ARCHIVE_DIGEST_0=", digest[0]);
    write_digest_word("AGENT_KERNEL_DURABLE_ARCHIVE_DIGEST_1=", digest[1]);
    write_digest_word("AGENT_KERNEL_DURABLE_ARCHIVE_DIGEST_2=", digest[2]);
    write_digest_word("AGENT_KERNEL_DURABLE_ARCHIVE_DIGEST_3=", digest[3]);
    serial_write_line("AGENT_KERNEL_DURABLE_ARCHIVE_COMMITTED_OK");
    pending.acknowledge_durable_archive_committed(call_data_generation, checkpoint)
}

pub(super) fn sign(
    booted: &mut X86BootedKernel,
    session: &mut NativeDurableSession<'_>,
    signer: &mut dyn KernelStateSigner,
    mut pending: PendingAgentCallCpu,
    call_data_generation: u64,
) -> Option<ResumableAgentCpu> {
    let current_request = pending.authenticated_signable_durable_archive_request()?;
    let context = pending.context();
    let caller = caller(context)?;
    let preparation = session.preparation()?;
    if preparation.caller() != caller
        || preparation.call_data_generation() != call_data_generation
        || !state::running(booted, context)
    {
        return None;
    }

    let kernel = booted.kernel();
    let entry = kernel.agent_entry(context.agent()).ok()?;
    let image = kernel.agent_image(context.image()).ok()?;
    if entry.kind != AgentEntryKind::StateSigner
        || entry.image != context.image()
        || image.kind != AgentImageKind::StateSigner
        || image.status != AgentImageStatus::Verified
    {
        return None;
    }

    let prepared_preflight = preparation.preflight();
    let current_preflight = kernel
        .preflight_durable_event_archive(
            context.agent(),
            prepared_preflight.archive_authority(),
            prepared_preflight.storage_authority(),
            session.config().storage(),
            prepared_preflight.proposal(),
        )
        .ok()?;
    if current_preflight != prepared_preflight {
        return None;
    }

    let configured_signer = session.config().signer();
    if configured_signer.status != DurableStateSignerStatus::Active
        || configured_signer.signature_algorithm() != DurableSignatureAlgorithm::EcdsaP256Sha256
        || configured_signer.signer_id != signer.signer_id()
        || configured_signer.generation != signer.policy_generation()
        || session.config().policy_generation() != signer.policy_generation()
    {
        return None;
    }

    let retained_request = preparation.request_bytes();
    let signed = sign_retained_durable_request(
        &retained_request,
        &current_request,
        preparation.manifest(),
        call_data_generation,
        signer,
    )
    .ok()?;
    if !pending.replace_signable_durable_archive_request(&current_request, &signed) {
        return None;
    }
    serial_write_line("AGENT_KERNEL_DURABLE_ARCHIVE_TPM_SIGNED_OK");
    pending.acknowledge_durable_archive_signed(call_data_generation, signer.policy_generation())
}

pub(super) fn cancel_for_context(
    session: &mut NativeDurableSession<'_>,
    context: AgentCallContext,
) -> Option<()> {
    let Some(preparation) = session.preparation() else {
        return Some(());
    };
    let caller = caller(context)?;
    if preparation.caller() != caller {
        return Some(());
    }
    session
        .cancel_preparation(caller, preparation.call_data_generation())
        .ok()?;
    serial_write_line("AGENT_KERNEL_DURABLE_ARCHIVE_PREPARATION_CANCELLED_OK");
    Some(())
}

fn caller(context: AgentCallContext) -> Option<NativeDurableArchiveCaller> {
    NativeDurableArchiveCaller::new(context.agent(), context.task(), context.image())
}

fn write_digest_word(marker: &str, word: u64) {
    serial_write_str(marker);
    serial_write_u64(word);
    serial_write_line("");
}
