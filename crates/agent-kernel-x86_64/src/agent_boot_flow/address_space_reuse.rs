//! End-to-end execution proof for one reclaimed native Agent address space.
//!
//! This x86 boot adapter joins semantic Reuse Worker admission with atomic
//! physical allocation, fixed page-table reconstruction, ring-3 execution,
//! verification, and terminal return of the same eleven frames.

use crate::{
    agent_cpu::AgentCpuRuntime,
    agent_memory::{
        NativeAddressSpaceFramePool, PreparedAgentMemory, RuntimeMemoryPool,
        NATIVE_ADDRESS_SPACE_FRAME_CAPACITY,
    },
    boot_agent_images::BootReuseWorkerImage,
    native_agent_executor::{self, NativeExecutionReport, NativeRuntimeEvidence},
    native_agent_runtime::NativeAgentRuntime,
    reuse_worker_flow::PreparedReuseWorkerFlow,
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
    let flow = PreparedReuseWorkerFlow::prepare(booted, contract)?;
    let verified_image = flow.verified_image(booted, contract.bytes())?;
    let frames = address_space_pool.allocate_zeroed()?;
    let identity = frames.identity();
    if address_space_pool.len()
        != NATIVE_ADDRESS_SPACE_FRAME_CAPACITY
            .checked_sub(agent_kernel_x86_64::address_space::AGENT_OWNED_FRAME_COUNT)?
        || address_space_pool.owns(identity)
    {
        return None;
    }
    serial_write_line("AGENT_KERNEL_NATIVE_ADDRESS_SPACE_ALLOCATED_OK");

    let memory = PreparedAgentMemory::prepare_reused(frames, verified_image)?;
    if memory.identity() != identity
        || !memory.kernel_address_space_active()
        || !memory_pool.is_disjoint_from(&memory)
        || address_space_pool.owns(identity)
    {
        return None;
    }
    serial_write_line("AGENT_KERNEL_NATIVE_ADDRESS_SPACE_REBUILT_OK");

    let cpu = cpu_runtime.prepare(memory, flow.call_context()?)?;
    if runtime.register_prepared(cpu).is_some() || runtime.len() != 1 {
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
    if !runtime.is_empty()
        || report.len() != 1
        || report.faulted_len() != 0
        || !evidence.proves_reuse_worker()
        || !flow.completed_after_runtime(booted, &report, contract)
    {
        return None;
    }
    flow.verify_completed(booted)?;
    serial_write_line("AGENT_KERNEL_NATIVE_ADDRESS_SPACE_REUSE_EXECUTION_OK");

    report.reclaim_completed_address_spaces(address_space_pool, [flow.agent()])?;
    if !address_space_pool.all_reclaimed_and_zero()
        || !address_space_pool.owns(identity)
        || !memory_pool.all_available_and_zero()
    {
        return None;
    }
    serial_write_line("AGENT_KERNEL_NATIVE_ADDRESS_SPACE_REUSED_RECLAIMED_OK");
    Some(())
}
