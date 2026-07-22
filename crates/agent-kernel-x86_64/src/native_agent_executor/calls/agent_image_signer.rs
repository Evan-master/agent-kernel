//! Native execution of one capability-authorized signer rotation.
//!
//! This binary-layer handler snapshots typed call data only after scheduled
//! identity authentication, delegates the atomic transition to the facade,
//! and reconciles retained records plus both replay Events before resuming.

use agent_kernel_core::{AgentImageSignerStatus, EventKind, Operation};
use agent_kernel_x86_64::typed_call_data::CallDataMessage;

use crate::{
    agent_cpu::{PendingAgentCallCpu, ResumableAgentCpu},
    native_agent_executor::state,
    serial_write_line, X86BootedKernel,
};

pub(super) fn rotate(
    booted: &mut X86BootedKernel,
    pending: PendingAgentCallCpu,
    generation: u64,
) -> Option<ResumableAgentCpu> {
    let CallDataMessage::RotateAgentImageSigner(message) =
        pending.authenticated_typed_call_data_message()?
    else {
        return None;
    };
    if message.generation() != generation {
        return None;
    }
    let context = pending.context();
    let authority = booted.kernel().capability(message.authority()).ok()?;
    let previous = booted
        .kernel()
        .agent_image_signer(message.previous_signer_id())
        .ok()?;
    let signer_count = booted.kernel().agent_image_signers().len();
    let event_start = booted.kernel().events().len();
    let next_sequence = booted.kernel().next_event_sequence();
    let rotation = booted
        .kernel_mut()
        .sys_rotate_agent_image_signer(
            context.agent(),
            message.authority(),
            authority.resource,
            message.expected_policy_generation(),
            message.previous_signer_id(),
            message.replacement_public_key(),
            message.replacement_image_kinds(),
            message.replacement_minimum_abi(),
            message.replacement_maximum_abi(),
        )
        .ok()?;
    let revoked = rotation.previous();
    let replacement = rotation.replacement();
    let kernel = booted.kernel();
    let events = kernel.events().get(event_start..)?;
    if authority.agent != context.agent()
        || previous.signer_id != revoked.signer_id
        || previous.status != AgentImageSignerStatus::Active
        || revoked.status != AgentImageSignerStatus::Revoked
        || replacement.status != AgentImageSignerStatus::Active
        || replacement.public_key != message.replacement_public_key()
        || replacement.image_kinds != message.replacement_image_kinds()
        || replacement.minimum_abi != message.replacement_minimum_abi()
        || replacement.maximum_abi != message.replacement_maximum_abi()
        || rotation.generation() != message.expected_policy_generation().checked_add(1)?
        || kernel.agent_image_signer_policy_generation() != rotation.generation()
        || kernel.agent_image_signers().len() != signer_count + 1
        || kernel.agent_image_signer(revoked.signer_id).ok()? != revoked
        || kernel.agent_image_signer(replacement.signer_id).ok()? != replacement
        || events.len() != 2
        || !valid_event(
            &events[0],
            next_sequence,
            EventKind::AgentImageSignerTrusted,
            Operation::Verify,
            context.agent(),
            message.authority(),
            replacement,
            Some(revoked.signer_id),
        )
        || !valid_event(
            &events[1],
            next_sequence + 1,
            EventKind::AgentImageSignerRevoked,
            Operation::Rollback,
            context.agent(),
            message.authority(),
            revoked,
            Some(replacement.signer_id),
        )
        || !state::running(booted, context)
    {
        return None;
    }
    serial_write_line("AGENT_KERNEL_AGENT_CALL_SIGNER_ROTATION_OK");
    pending.acknowledge_agent_image_signer_rotation(rotation, message, signer_count + 1)
}

#[allow(clippy::too_many_arguments)]
fn valid_event(
    event: &agent_kernel_core::Event,
    sequence: u64,
    kind: EventKind,
    operation: Operation,
    actor: agent_kernel_core::AgentId,
    authority: agent_kernel_core::CapabilityId,
    record: agent_kernel_core::AgentImageSignerRecord,
    peer: Option<agent_kernel_core::AgentImageSignerId>,
) -> bool {
    event.sequence == sequence
        && event.kind == kind
        && event.agent == actor
        && event.resource == Some(record.resource)
        && event.capability == Some(authority)
        && event.operation == Some(operation)
        && event.agent_image_signer.is_some_and(|evidence| {
            evidence.signer_id == record.signer_id
                && evidence.peer_signer_id == peer
                && evidence.public_key == record.public_key
                && evidence.image_kinds == record.image_kinds
                && evidence.minimum_abi == record.minimum_abi
                && evidence.maximum_abi == record.maximum_abi
                && evidence.status == record.status
                && evidence.policy_generation == record.generation
        })
}
