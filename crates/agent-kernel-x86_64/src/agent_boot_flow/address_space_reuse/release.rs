//! Verified physical reclamation and semantic release for admission batches.
//!
//! This boot-flow child prepares opaque release permits before moving frames,
//! commits each permit only after exact zeroed ownership returns, and preserves
//! the resident Supervisor through the partial first-batch boundary.

use agent_kernel_x86_64::address_space::AGENT_OWNED_FRAME_COUNT;

use crate::{
    admission_supervisor_flow::{PreparedAdmissionSupervisorFlow, ADMISSION_SUPERVISOR},
    agent_memory::{
        NativeAddressSpaceFramePool, RuntimeMemoryPool, NATIVE_ADDRESS_SPACE_FRAME_CAPACITY,
    },
    native_address_space_service::NativeAddressSpaceAdmission,
    native_agent_executor::NativeExecutionReport,
    native_agent_runtime::NativeAgentRuntime,
    reuse_worker_flow::PreparedReuseWorkerFlow,
    serial_write_line, X86BootedKernel,
};

#[allow(clippy::too_many_arguments)]
pub(super) fn partial(
    booted: &mut X86BootedKernel,
    report: &mut NativeExecutionReport,
    pool: &mut NativeAddressSpaceFramePool,
    runtime: &NativeAgentRuntime,
    memory_pool: &RuntimeMemoryPool,
    supervisor: &PreparedAdmissionSupervisorFlow,
    flows: &[PreparedReuseWorkerFlow; 2],
    supervisor_admission: NativeAddressSpaceAdmission,
    admissions: [NativeAddressSpaceAdmission; 2],
) -> Option<()> {
    for flow in flows {
        flow.verify_completed(booted)?;
    }
    let release_ids = [
        booted.kernel().runtime_admissions().first()?.id,
        booted.kernel().runtime_admissions().get(1)?.id,
    ];
    let release = booted
        .kernel()
        .sys_prepare_runtime_admission_release_batch(release_ids)
        .ok()?;
    let event_start = booted.kernel().events().len();
    let targets = [flows[0].admission_target(), flows[1].admission_target()];
    report.reclaim_completed_address_spaces(pool, [targets[0].0, targets[1].0])?;
    if report.len() != 0
        || report.faulted_len() != 0
        || pool.len() + AGENT_OWNED_FRAME_COUNT != NATIVE_ADDRESS_SPACE_FRAME_CAPACITY
        || !pool.owns_zeroed(admissions[0].identity())
        || !pool.owns_zeroed(admissions[1].identity())
        || pool.owns(supervisor_admission.identity())
        || runtime.len() != 1
        || !runtime.contains(ADMISSION_SUPERVISOR)
        || !memory_pool.all_available_and_zero()
        || booted.kernel().events().len() != event_start
    {
        return None;
    }
    serial_write_line("AGENT_KERNEL_NATIVE_ADDRESS_SPACE_PARTIAL_RECLAIM_OK");
    booted
        .kernel_mut()
        .sys_commit_runtime_admission_release_batch(release)
        .ok()?;
    if !supervisor.released_batch_after_reclamation(booted, targets, 0, event_start) {
        return None;
    }
    serial_write_line("AGENT_KERNEL_NATIVE_RUNTIME_ADMISSION_RELEASE_OK");
    Some(())
}

#[allow(clippy::too_many_arguments)]
pub(super) fn terminal(
    booted: &mut X86BootedKernel,
    report: &mut NativeExecutionReport,
    pool: &mut NativeAddressSpaceFramePool,
    runtime: &NativeAgentRuntime,
    memory_pool: &RuntimeMemoryPool,
    supervisor: &PreparedAdmissionSupervisorFlow,
    flows: &[PreparedReuseWorkerFlow; 2],
    supervisor_admission: NativeAddressSpaceAdmission,
    admissions: [NativeAddressSpaceAdmission; 2],
) -> Option<()> {
    for flow in flows {
        flow.verify_completed(booted)?;
    }
    supervisor.verify_completed(booted)?;
    let release_ids = [
        booted.kernel().runtime_admissions().get(2)?.id,
        booted.kernel().runtime_admissions().get(3)?.id,
    ];
    let release = booted
        .kernel()
        .sys_prepare_runtime_admission_release_batch(release_ids)
        .ok()?;
    let event_start = booted.kernel().events().len();
    let targets = [flows[0].admission_target(), flows[1].admission_target()];
    report.reclaim_completed_address_spaces(
        pool,
        [ADMISSION_SUPERVISOR, targets[0].0, targets[1].0],
    )?;
    if report.len() != 0
        || report.faulted_len() != 0
        || !runtime.is_empty()
        || !pool.all_reclaimed_and_zero()
        || !pool.owns_zeroed(supervisor_admission.identity())
        || !pool.owns_zeroed(admissions[0].identity())
        || !pool.owns_zeroed(admissions[1].identity())
        || !memory_pool.all_available_and_zero()
        || booted.kernel().events().len() != event_start
    {
        return None;
    }
    serial_write_line("AGENT_KERNEL_NATIVE_ADDRESS_SPACE_REUSED_RECLAIMED_OK");
    booted
        .kernel_mut()
        .sys_commit_runtime_admission_release_batch(release)
        .ok()?;
    if !supervisor.released_batch_after_reclamation(booted, targets, 2, event_start) {
        return None;
    }
    serial_write_line("AGENT_KERNEL_NATIVE_RUNTIME_ADMISSION_RELEASE_OK");
    serial_write_line("AGENT_KERNEL_NATIVE_RUNTIME_ADMISSION_REPEAT_OK");
    Some(())
}
