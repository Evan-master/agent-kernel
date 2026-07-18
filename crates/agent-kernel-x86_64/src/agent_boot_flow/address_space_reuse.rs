//! Transactional batch execution on reclaimed native Agent address spaces.
//!
//! Two semantic Workers receive disjoint private frame sets, remain parked in
//! the native runtime together, execute under core FIFO policy, and return all
//! physical ownership. A duplicate admission proves post-build cancellation.

use agent_kernel_x86_64::address_space::AGENT_OWNED_FRAME_COUNT;

use crate::{
    agent_cpu::AgentCpuRuntime,
    agent_memory::{
        NativeAddressSpaceFramePool, RuntimeMemoryPool, NATIVE_ADDRESS_SPACE_FRAME_CAPACITY,
    },
    boot_agent_images::BootReuseWorkerImage,
    native_address_space_service::{NativeAddressSpaceAdmissionStage, NativeAddressSpaceService},
    native_agent_executor::{self, NativeExecutionReport, NativeRuntimeEvidence},
    native_agent_runtime::NativeAgentRuntime,
    reuse_worker_flow::{PreparedReuseWorkerFlow, REUSE_WORKERS},
    serial_write_line, X86BootedKernel,
};

pub(super) fn run(
    booted: &mut X86BootedKernel,
    runtime: &mut NativeAgentRuntime,
    memory_pool: &mut RuntimeMemoryPool,
    address_space_pool: &mut NativeAddressSpaceFramePool,
    cpu_runtime: &AgentCpuRuntime,
    contract: BootReuseWorkerImage,
) -> Option<()> {
    if !runtime.is_empty()
        || !memory_pool.all_available_and_zero()
        || !address_space_pool.all_reclaimed_and_zero()
    {
        return None;
    }

    let first = PreparedReuseWorkerFlow::prepare(booted, REUSE_WORKERS[0], contract)?;
    let second = PreparedReuseWorkerFlow::prepare(booted, REUSE_WORKERS[1], contract)?;
    let flows = [first, second];
    if !PreparedReuseWorkerFlow::batch_queued(booted, &flows) {
        return None;
    }
    let images = [
        flows[0].verified_image(booted, contract.bytes())?,
        flows[1].verified_image(booted, contract.bytes())?,
    ];
    let contexts = [flows[0].call_context()?, flows[1].call_context()?];

    let first_admission = NativeAddressSpaceService::admit(
        address_space_pool,
        runtime,
        cpu_runtime,
        memory_pool,
        images[0],
        contexts[0],
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
        images[0],
        contexts[0],
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

    let second_admission = NativeAddressSpaceService::admit(
        address_space_pool,
        runtime,
        cpu_runtime,
        memory_pool,
        images[1],
        contexts[1],
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
    {
        return None;
    }
    serial_write_line("AGENT_KERNEL_NATIVE_ADDRESS_SPACE_ALLOCATED_OK");
    serial_write_line("AGENT_KERNEL_NATIVE_ADDRESS_SPACE_REBUILT_OK");
    serial_write_line("AGENT_KERNEL_NATIVE_ADDRESS_SPACE_RUNTIME_BATCH_OK");
    serial_write_line("AGENT_KERNEL_NATIVE_ADDRESS_SPACE_RUNTIME_CONCURRENCY_OK");

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
            .any(|flow| !flow.completed_after_runtime(booted, &report, contract))
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
