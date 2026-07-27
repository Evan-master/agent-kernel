//! Durable writer admission and execution for the native StateSigner Agent.
//!
//! This boot-flow child binds public kernel authority to one signed ring3
//! package, executes it from reclaimed frames, and returns the committed Event
//! archive proof. TPM commands remain behind the kernel signer service.

mod evidence;
mod setup;

use agent_kernel_core::{AgentImageId, CapabilityId, IntentId, TaskId};
use agent_kernel_x86_64::{
    agent_call::AgentCallContext, ata::AtaDurableHead, tpm2::KernelStateSigner,
};

use crate::{
    agent_cpu::AgentCpuRuntime,
    agent_memory::{NativeAddressSpaceFramePool, RuntimeMemoryPool},
    boot_config::StateSignerBootProfile,
    native_address_space_service::NativeAddressSpaceService,
    native_agent_executor::{
        self, NativeEventArchive, NativeExecutionReport, NativeRuntimeEvidence,
    },
    native_agent_runtime::NativeAgentRuntime,
    serial_write_line,
    smp_boot::SmpBootstrap,
    NativeDurableSession, X86BootedKernel,
};

#[derive(Copy, Clone)]
struct PreparedStateSigner {
    intent: IntentId,
    task: TaskId,
    image: AgentImageId,
    task_authority: CapabilityId,
}

impl PreparedStateSigner {
    const fn context(self, profile: StateSignerBootProfile) -> Option<AgentCallContext> {
        AgentCallContext::new(profile.agent(), self.task, self.image, self.task_authority)
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) fn run(
    booted: &mut X86BootedKernel,
    runtime: &mut NativeAgentRuntime,
    memory_pool: &mut RuntimeMemoryPool,
    address_space_pool: &mut NativeAddressSpaceFramePool,
    smp_bootstrap: &mut SmpBootstrap,
    cpu_runtime: &AgentCpuRuntime,
    session: &mut NativeDurableSession<'_>,
    state_signer: &mut Option<&mut dyn KernelStateSigner>,
    bootstrap_storage_authority: CapabilityId,
    profile: StateSignerBootProfile,
    retained_snapshot: &NativeEventArchive,
) -> Option<NativeEventArchive> {
    let inventory = address_space_pool.inventory_frame_count()?;
    if !runtime.is_empty()
        || !booted.kernel().run_queue().is_empty()
        || !memory_pool.all_available_and_zero()
        || !address_space_pool.all_reclaimed_and_zero()
        || address_space_pool.len() != inventory
        || state_signer.is_none()
        || !retained_snapshot.is_retained_snapshot()
        || retained_snapshot.proposal()?.through_sequence() != profile.through_sequence()
    {
        return None;
    }
    serial_write_line("AGENT_KERNEL_NATIVE_STATE_SIGNER_PRECONDITIONS_OK");

    let prepared = setup::prepare(
        booted,
        bootstrap_storage_authority,
        profile,
        session.config().storage(),
    )?;
    let context = prepared.context(profile)?;
    let verified_image = setup::verified_image(booted, prepared, profile)?;
    serial_write_line("AGENT_KERNEL_NATIVE_STATE_SIGNER_IMAGE_ADMITTED_OK");
    setup::queue(booted, prepared, profile)?;
    serial_write_line("AGENT_KERNEL_NATIVE_STATE_SIGNER_QUEUED_OK");
    let admission = NativeAddressSpaceService::admit(
        address_space_pool,
        runtime,
        cpu_runtime,
        memory_pool,
        verified_image,
        context,
    )?
    .ok()?;
    if admission.agent() != profile.agent()
        || address_space_pool.len() + admission.identity().owned_frame_count() != inventory
    {
        return None;
    }
    serial_write_line("AGENT_KERNEL_NATIVE_STATE_SIGNER_ADDRESS_SPACE_ADMITTED_OK");

    let mut report = NativeExecutionReport::new();
    let mut runtime_evidence = NativeRuntimeEvidence::default();
    native_agent_executor::run_until_idle(
        booted,
        runtime,
        memory_pool,
        &mut report,
        &mut runtime_evidence,
        None,
        Some(session),
        state_signer,
    )?;
    serial_write_line("AGENT_KERNEL_NATIVE_STATE_SIGNER_EXECUTION_RETURNED_OK");
    if !runtime.is_empty()
        || report.len() != 1
        || report.faulted_len() != 0
        || !runtime_evidence.proves_state_signer_flow()
        || !evidence::completed(
            booted,
            &report,
            prepared,
            profile,
            retained_snapshot,
            session,
        )
    {
        return None;
    }

    report.reclaim_completed_address_spaces(
        address_space_pool,
        smp_bootstrap,
        [profile.agent()],
    )?;
    if !address_space_pool.all_reclaimed_and_zero()
        || address_space_pool.len() != inventory
        || !memory_pool.all_available_and_zero()
        || session.preparation().is_some()
        || session.is_faulted()
        || session.backend().head() != Some(AtaDurableHead::Recovered(1))
    {
        return None;
    }
    serial_write_line("AGENT_KERNEL_NATIVE_STATE_SIGNER_ADDRESS_SPACE_RECLAIMED_OK");
    Some(report.into_event_archive())
}
