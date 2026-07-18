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
    let targets = [flows[0].admission_target(), flows[1].admission_target()];
    if runtime.len() != 1
        || !runtime.contains(ADMISSION_SUPERVISOR)
        || report.len() != 0
        || report.faulted_len() != 0
        || !evidence.proves_runtime_admission_wait()
        || !supervisor.waiting_after_requests(booted, targets)
        || address_space_pool.len()
            != NATIVE_ADDRESS_SPACE_FRAME_CAPACITY.checked_sub(AGENT_OWNED_FRAME_COUNT)?
        || address_space_pool.owns(supervisor_admission.identity())
        || !memory_pool.all_available_and_zero()
    {
        return None;
    }
    serial_write_line("AGENT_KERNEL_NATIVE_RUNTIME_ADMISSION_REQUEST_OK");
    serial_write_line("AGENT_KERNEL_NATIVE_RUNTIME_ADMISSION_RESIDENT_WAIT_OK");

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
            != NATIVE_ADDRESS_SPACE_FRAME_CAPACITY.checked_sub(2 * AGENT_OWNED_FRAME_COUNT)?
        || !supervisor_admission.is_disjoint_from(first_admission)
        || runtime.len() != 2
        || !runtime.contains(ADMISSION_SUPERVISOR)
        || !runtime.contains(REUSE_WORKERS[0])
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
        != NATIVE_ADDRESS_SPACE_FRAME_CAPACITY.checked_sub(2 * AGENT_OWNED_FRAME_COUNT)?
        || !address_space_pool.owns_zeroed(cancelled_identity)
        || runtime.len() != 2
        || !runtime.contains(ADMISSION_SUPERVISOR)
        || !runtime.contains(REUSE_WORKERS[0])
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
        || !supervisor_admission.is_disjoint_from(second_admission)
        || !first_admission.is_disjoint_from(second_admission)
        || address_space_pool.len()
            != NATIVE_ADDRESS_SPACE_FRAME_CAPACITY.checked_sub(3 * AGENT_OWNED_FRAME_COUNT)?
        || address_space_pool.owns(supervisor_admission.identity())
        || address_space_pool.owns(first_admission.identity())
        || address_space_pool.owns(second_admission.identity())
        || runtime.len() != 3
        || !runtime.contains(ADMISSION_SUPERVISOR)
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

    native_agent_executor::run_until_idle(
        booted,
        runtime,
        memory_pool,
        &mut report,
        &mut evidence,
        None,
    )?;
    if !runtime.is_empty()
        || report.len() != 3
        || report.faulted_len() != 0
        || !evidence.proves_resident_runtime_admission_flow()
        || flows
            .iter()
            .any(|flow| !flow.completed_after_runtime(booted, &report, worker_contract))
        || !supervisor.completed_after_notifications(booted, &report, supervisor_contract, targets)
    {
        return None;
    }
    serial_write_line("AGENT_KERNEL_NATIVE_RUNTIME_ADMISSION_NOTIFICATION_OK");
    for flow in &flows {
        flow.verify_completed(booted)?;
    }
    supervisor.verify_completed(booted)?;
    serial_write_line("AGENT_KERNEL_NATIVE_RUNTIME_ADMISSION_SUPERVISOR_OK");
    serial_write_line("AGENT_KERNEL_NATIVE_ADDRESS_SPACE_REUSE_EXECUTION_OK");

    report.reclaim_completed_address_spaces(
        address_space_pool,
        [ADMISSION_SUPERVISOR, REUSE_WORKERS[0], REUSE_WORKERS[1]],
    )?;
    if !address_space_pool.all_reclaimed_and_zero()
        || !address_space_pool.owns(supervisor_admission.identity())
        || !address_space_pool.owns(first_admission.identity())
        || !address_space_pool.owns(second_admission.identity())
        || !memory_pool.all_available_and_zero()
    {
        return None;
    }
    serial_write_line("AGENT_KERNEL_NATIVE_ADDRESS_SPACE_REUSED_RECLAIMED_OK");
    Some(())
}
