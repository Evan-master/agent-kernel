//! Public-kernel admission and signed-image verification for StateSigner.

use agent_kernel_core::{
    agent_image_signer_id, AgentEntryKind, AgentExecutionState, AgentImageKind,
    AgentImageKindScope, AgentImageSignerStatus, AgentImageStatus, AgentStatus, EventKind,
    IntentKind, Operation, OperationSet, ResourceId, RunQueueEntry, TaskStatus,
    VerificationRequirement,
};
use agent_kernel_x86_64::agent_image::{
    sha256_digest, AgentImageCapsule, AgentImageFormat, AgentImageTrust, AgentImageTrustPolicy,
    VerifiedAgentImage,
};

use super::PreparedStateSigner;
use crate::{
    boot_config::StateSignerBootProfile, serial_write_line, serial_write_str, serial_write_u64,
    X86BootedKernel,
};

pub(super) fn prepare(
    booted: &mut X86BootedKernel,
    bootstrap_storage_authority: agent_kernel_core::CapabilityId,
    profile: StateSignerBootProfile,
    storage: ResourceId,
) -> Option<PreparedStateSigner> {
    let report = *booted.report();
    let expected_signer = agent_image_signer_id(profile.public_key());
    let kernel = booted.kernel_mut();
    let trusted = kernel
        .sys_trust_agent_image_signer(
            report.bootstrap_agent,
            report.bootstrap_capability,
            report.bootstrap_resource,
            profile.public_key(),
            AgentImageKindScope::only(AgentImageKind::StateSigner),
            1,
            1,
        )
        .ok()?;
    if trusted.signer_id != expected_signer
        || trusted.status != AgentImageSignerStatus::Active
        || trusted.resource != report.bootstrap_resource
    {
        return None;
    }
    serial_write_line("AGENT_KERNEL_NATIVE_STATE_SIGNER_TRUST_OK");

    write_id(
        "AGENT_KERNEL_NATIVE_STATE_SIGNER_AGENT_RECORDS_BEFORE=",
        kernel.agents().len() as u64,
    );
    let agent = kernel
        .agents()
        .iter()
        .find(|record| record.id == profile.agent())?;
    let execution = kernel
        .execution_contexts()
        .iter()
        .find(|context| context.agent == profile.agent())?;
    if agent.status != AgentStatus::Active
        || execution.state != AgentExecutionState::Idle
        || execution.task.is_some()
        || execution.driver_invocation.is_some()
        || kernel.agent_entry(profile.agent()).is_ok()
    {
        return None;
    }
    serial_write_line("AGENT_KERNEL_NATIVE_STATE_SIGNER_AGENT_REUSED_OK");
    let intent = kernel
        .sys_declare_intent(
            report.bootstrap_agent,
            report.bootstrap_capability,
            report.bootstrap_resource,
            IntentKind::Act,
            VerificationRequirement::Required,
        )
        .ok()?;
    let task = kernel
        .sys_create_task(report.bootstrap_agent, report.bootstrap_capability, intent)
        .ok()?;
    kernel
        .sys_delegate_task(
            report.bootstrap_agent,
            report.bootstrap_capability,
            task,
            profile.agent(),
        )
        .ok()?;
    serial_write_line("AGENT_KERNEL_NATIVE_STATE_SIGNER_TASK_READY_OK");
    let task_authority = kernel
        .tasks()
        .iter()
        .find(|record| record.id == task)?
        .delegated_capability?;
    let archive_authority = kernel
        .sys_derive_capability(
            report.bootstrap_agent,
            report.bootstrap_capability,
            profile.agent(),
            OperationSet::only(Operation::Rollback),
        )
        .ok()?;
    let storage_authority = kernel
        .sys_derive_capability(
            report.bootstrap_agent,
            bootstrap_storage_authority,
            profile.agent(),
            OperationSet::only(Operation::Checkpoint),
        )
        .ok()?;
    if archive_authority != profile.archive_authority()
        || storage_authority != profile.storage_authority()
    {
        write_id(
            "AGENT_KERNEL_STATE_SIGNER_ACTUAL_ARCHIVE_AUTHORITY=",
            archive_authority.raw(),
        );
        write_id(
            "AGENT_KERNEL_STATE_SIGNER_ACTUAL_STORAGE_AUTHORITY=",
            storage_authority.raw(),
        );
        return None;
    }
    serial_write_line("AGENT_KERNEL_NATIVE_STATE_SIGNER_AUTHORITIES_OK");

    let capsule = AgentImageCapsule::parse(profile.package()).ok()?;
    if capsule.format() != AgentImageFormat::SignedPackageV3
        || capsule.signer_id() != Some(expected_signer)
    {
        return None;
    }
    let image = kernel
        .sys_register_agent_image(
            report.bootstrap_agent,
            report.bootstrap_capability,
            report.bootstrap_resource,
            AgentImageKind::StateSigner,
            sha256_digest(profile.package()),
            1,
            1,
        )
        .ok()?;
    kernel
        .sys_verify_agent_image(report.bootstrap_agent, report.bootstrap_capability, image)
        .ok()?;
    serial_write_line("AGENT_KERNEL_NATIVE_STATE_SIGNER_PACKAGE_VERIFIED_OK");
    kernel
        .sys_launch_task_agent(
            profile.agent(),
            task_authority,
            task,
            image,
            AgentEntryKind::StateSigner,
        )
        .ok()?;
    kernel.sys_accept_task(profile.agent(), task).ok()?;
    serial_write_line("AGENT_KERNEL_NATIVE_STATE_SIGNER_LAUNCHED_OK");

    let prepared = PreparedStateSigner {
        intent,
        task,
        image,
        task_authority,
    };
    if !prepared_state_valid(booted, prepared, profile, storage) {
        return None;
    }
    serial_write_line("AGENT_KERNEL_NATIVE_STATE_SIGNER_PREPARED_OK");
    Some(prepared)
}

pub(super) fn verified_image(
    booted: &X86BootedKernel,
    prepared: PreparedStateSigner,
    profile: StateSignerBootProfile,
) -> Option<VerifiedAgentImage<'static>> {
    let policy = AgentImageTrustPolicy::new(booted.kernel().agent_image_signers());
    let verified = VerifiedAgentImage::verify_signed(
        booted.kernel().agent_image(prepared.image).ok()?,
        profile.package(),
        &policy,
    )
    .ok()?;
    (verified.format() == AgentImageFormat::SignedPackageV3
        && verified.signer_id() == Some(agent_image_signer_id(profile.public_key()))
        && verified.trust() == AgentImageTrust::Signed(agent_image_signer_id(profile.public_key())))
    .then_some(verified)
}

pub(super) fn queue(
    booted: &mut X86BootedKernel,
    prepared: PreparedStateSigner,
    profile: StateSignerBootProfile,
) -> Option<()> {
    if !booted.kernel().run_queue().is_empty() {
        return None;
    }
    let event = booted
        .kernel_mut()
        .sys_enqueue_task(profile.agent(), prepared.task)
        .ok()?;
    (event.kind == EventKind::TaskQueued
        && event.agent == profile.agent()
        && event.task == Some(prepared.task)
        && booted.kernel().run_queue()
            == [RunQueueEntry {
                task: prepared.task,
                agent: profile.agent(),
            }])
    .then_some(())
}

fn prepared_state_valid(
    booted: &X86BootedKernel,
    prepared: PreparedStateSigner,
    profile: StateSignerBootProfile,
    storage: ResourceId,
) -> bool {
    let kernel = booted.kernel();
    let task = kernel
        .tasks()
        .iter()
        .find(|record| record.id == prepared.task);
    let image = kernel.agent_image(prepared.image).ok();
    let archive = kernel.capability(profile.archive_authority()).ok();
    let storage_capability = kernel.capability(profile.storage_authority()).ok();
    matches!(task, Some(task)
        if task.status == TaskStatus::Accepted
            && task.delegated_capability == Some(prepared.task_authority)
            && task.result.is_none())
        && matches!(image, Some(image)
            if image.kind == AgentImageKind::StateSigner
                && image.status == AgentImageStatus::Verified)
        && matches!(archive, Some(capability)
            if capability.agent == profile.agent()
                && capability.operations == OperationSet::only(Operation::Rollback)
                && !capability.revoked)
        && matches!(storage_capability, Some(capability)
            if capability.agent == profile.agent()
                && capability.resource == storage
                && capability.operations == OperationSet::only(Operation::Checkpoint)
                && !capability.revoked)
}

fn write_id(marker: &str, value: u64) {
    serial_write_str(marker);
    serial_write_u64(value);
    serial_write_line("");
}
