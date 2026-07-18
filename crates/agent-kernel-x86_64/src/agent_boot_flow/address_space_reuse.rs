//! Runtime Admission Supervisor and broker execution on reclaimed address spaces.

use agent_kernel_x86_64::address_space::AGENT_OWNED_FRAME_COUNT;

use crate::{
    admission_supervisor_flow::{PreparedAdmissionSupervisorFlow, ADMISSION_SUPERVISOR},
    agent_cpu::AgentCpuRuntime,
    agent_memory::{
        NativeAddressSpaceFramePool, RuntimeMemoryPool, NATIVE_ADDRESS_SPACE_FRAME_CAPACITY,
    },
    boot_agent_images::{BootAdmissionSupervisorImage, BootReuseWorkerImage},
    native_address_space_service::{NativeAddressSpaceAdmissionStage, NativeAddressSpaceService},
    native_agent_executor::{self, NativeExecutionReport, NativeRuntimeEvidence},
    native_agent_runtime::NativeAgentRuntime,
    native_runtime_admission_broker::NativeRuntimeAdmissionBroker,
    reuse_worker_flow::{PreparedReuseWorkerFlow, REUSE_WORKERS},
    serial_write_line, X86BootedKernel,
};

pub(super) fn run(
    booted: &mut X86BootedKernel,
    runtime: &mut NativeAgentRuntime,
    memory_pool: &mut RuntimeMemoryPool,
    address_space_pool: &mut NativeAddressSpaceFramePool,
    cpu_runtime: &AgentCpuRuntime,
    worker_contract: BootReuseWorkerImage,
    supervisor_contract: BootAdmissionSupervisorImage,
) -> Option<()> {
    if !runtime.is_empty()
        || !memory_pool.all_available_and_zero()
        || !address_space_pool.all_reclaimed_and_zero()
    {
        return None;
    }

    let first =
        PreparedReuseWorkerFlow::prepare_unqueued(booted, REUSE_WORKERS[0], worker_contract)?;
    let second =
        PreparedReuseWorkerFlow::prepare_unqueued(booted, REUSE_WORKERS[1], worker_contract)?;
    let flows = [first, second];
    if !PreparedReuseWorkerFlow::batch_unqueued(booted, &flows) {
        return None;
    }
    let supervisor =
        PreparedAdmissionSupervisorFlow::prepare(booted, supervisor_contract.digest())?;
    let supervisor_admission = NativeAddressSpaceService::admit(
        address_space_pool,
        runtime,
        cpu_runtime,
        memory_pool,
        supervisor.verified_image(booted, supervisor_contract.bytes())?,
        supervisor.call_context()?,
    )?
    .ok()?;
    if supervisor_admission.agent() != ADMISSION_SUPERVISOR
        || address_space_pool.len()
            != NATIVE_ADDRESS_SPACE_FRAME_CAPACITY.checked_sub(AGENT_OWNED_FRAME_COUNT)?
        || runtime.len() != 1
    {
        return None;
    }
    let mut supervisor_report = NativeExecutionReport::new();
    let mut supervisor_evidence = NativeRuntimeEvidence::default();
    native_agent_executor::run_until_idle(
        booted,
        runtime,
        memory_pool,
        &mut supervisor_report,
        &mut supervisor_evidence,
        None,
    )?;
    let targets = [flows[0].admission_target(), flows[1].admission_target()];
    if !runtime.is_empty()
        || supervisor_report.len() != 1
        || supervisor_report.faulted_len() != 0
        || !supervisor_evidence.proves_runtime_admission_supervisor()
        || !supervisor.completed_after_runtime(
            booted,
            &supervisor_report,
            supervisor_contract,
            targets,
        )
    {
        return None;
    }
    supervisor.verify_completed(booted)?;
    supervisor_report
        .reclaim_completed_address_spaces(address_space_pool, [ADMISSION_SUPERVISOR])?;
    if !address_space_pool.all_reclaimed_and_zero() || !memory_pool.all_available_and_zero() {
        return None;
    }
    serial_write_line("AGENT_KERNEL_NATIVE_RUNTIME_ADMISSION_REQUEST_OK");
    serial_write_line("AGENT_KERNEL_NATIVE_RUNTIME_ADMISSION_SUPERVISOR_OK");

    let first_admission = NativeRuntimeAdmissionBroker::admit_next(
        booted,
        address_space_pool,
        runtime,
        cpu_runtime,
        memory_pool,
        worker_contract.bytes(),
    )?
    .ok()?;
    if first_admission.agent() != REUSE_WORKERS[0]
        || address_space_pool.len()
            != NATIVE_ADDRESS_SPACE_FRAME_CAPACITY.checked_sub(AGENT_OWNED_FRAME_COUNT)?
        || runtime.len() != 1
    {
        return None;
    }

    let duplicate_failure = NativeAddressSpaceService::admit(
        address_space_pool,
        runtime,
        cpu_runtime,
        memory_pool,
        flows[0].verified_image(booted, worker_contract.bytes())?,
        flows[0].call_context()?,
    )?
    .err()?;
    let cancelled_identity = duplicate_failure.identity()?;
    if !duplicate_failure.proves_rollback(
        NativeAddressSpaceAdmissionStage::RuntimeRegistration,
        REUSE_WORKERS[0],
        cancelled_identity,
    ) || address_space_pool.len()
        != NATIVE_ADDRESS_SPACE_FRAME_CAPACITY.checked_sub(AGENT_OWNED_FRAME_COUNT)?
        || !address_space_pool.owns_zeroed(cancelled_identity)
        || runtime.len() != 1
    {
        return None;
    }
    serial_write_line("AGENT_KERNEL_NATIVE_ADDRESS_SPACE_RUNTIME_CANCEL_OK");

    let second_admission = NativeRuntimeAdmissionBroker::admit_next(
        booted,
        address_space_pool,
        runtime,
        cpu_runtime,
        memory_pool,
        worker_contract.bytes(),
    )?
    .ok()?;
    if second_admission.agent() != REUSE_WORKERS[1]
        || second_admission.identity() != cancelled_identity
        || !first_admission.is_disjoint_from(second_admission)
        || address_space_pool.len()
            != NATIVE_ADDRESS_SPACE_FRAME_CAPACITY.checked_sub(2 * AGENT_OWNED_FRAME_COUNT)?
        || address_space_pool.owns(first_admission.identity())
        || address_space_pool.owns(second_admission.identity())
        || runtime.len() != 2
        || !runtime.contains(REUSE_WORKERS[0])
        || !runtime.contains(REUSE_WORKERS[1])
        || !PreparedReuseWorkerFlow::batch_queued(booted, &flows)
    {
        return None;
    }
    serial_write_line("AGENT_KERNEL_NATIVE_ADDRESS_SPACE_ALLOCATED_OK");
    serial_write_line("AGENT_KERNEL_NATIVE_ADDRESS_SPACE_REBUILT_OK");
    serial_write_line("AGENT_KERNEL_NATIVE_ADDRESS_SPACE_RUNTIME_BATCH_OK");
    serial_write_line("AGENT_KERNEL_NATIVE_ADDRESS_SPACE_RUNTIME_CONCURRENCY_OK");
    serial_write_line("AGENT_KERNEL_NATIVE_RUNTIME_ADMISSION_COMMIT_OK");

    let mut report = NativeExecutionReport::new();
    let mut evidence = NativeRuntimeEvidence::default();
    native_agent_executor::run_until_idle(
        booted,
        runtime,
        memory_pool,
        &mut report,
        &mut evidence,
        None,
    )?;
    if !runtime.is_empty()
        || report.len() != 2
        || report.faulted_len() != 0
        || !evidence.proves_address_space_runtime_batch()
        || flows
            .iter()
            .any(|flow| !flow.completed_after_runtime(booted, &report, worker_contract))
    {
        return None;
    }
    for flow in &flows {
        flow.verify_completed(booted)?;
    }
    serial_write_line("AGENT_KERNEL_NATIVE_ADDRESS_SPACE_REUSE_EXECUTION_OK");

    report.reclaim_completed_address_spaces(address_space_pool, REUSE_WORKERS)?;
    if !address_space_pool.all_reclaimed_and_zero()
        || !address_space_pool.owns(first_admission.identity())
        || !address_space_pool.owns(second_admission.identity())
        || !memory_pool.all_available_and_zero()
    {
        return None;
    }
    serial_write_line("AGENT_KERNEL_NATIVE_ADDRESS_SPACE_REUSED_RECLAIMED_OK");
    Some(())
}
