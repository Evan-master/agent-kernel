//! Physical and semantic admission of one fixed two-Worker batch.
//!
//! This boot-flow child drives the generation-bound broker, proves complete
//! rollback for the retained first-batch cancellation probe, and validates
//! disjoint frame ownership before exposing the queued batch.

use agent_kernel_x86_64::address_space::AgentMemoryIdentity;

use crate::{
    admission_supervisor_flow::ADMISSION_SUPERVISOR,
    agent_cpu::AgentCpuRuntime,
    agent_memory::{NativeAddressSpaceFramePool, RuntimeMemoryPool},
    boot_agent_images::BootReuseWorkerImage,
    native_address_space_service::{
        NativeAddressSpaceAdmission, NativeAddressSpaceAdmissionStage, NativeAddressSpaceService,
    },
    native_agent_runtime::NativeAgentRuntime,
    native_runtime_admission_broker::NativeRuntimeAdmissionBroker,
    reuse_worker_flow::PreparedReuseWorkerFlow,
    serial_write_line, X86BootedKernel,
};

#[allow(clippy::too_many_arguments)]
pub(super) fn admit(
    booted: &mut X86BootedKernel,
    pool: &mut NativeAddressSpaceFramePool,
    runtime: &mut NativeAgentRuntime,
    cpu_runtime: &AgentCpuRuntime,
    memory_pool: &RuntimeMemoryPool,
    contract: BootReuseWorkerImage,
    flows: &[PreparedReuseWorkerFlow; 2],
    resident: NativeAddressSpaceAdmission,
    expected_identities: Option<[AgentMemoryIdentity; 2]>,
    prove_cancellation: bool,
) -> Option<[NativeAddressSpaceAdmission; 2]> {
    let initial_pool_len = pool.len();
    let initial_runtime_len = runtime.len();
    let first_target = flows[0].admission_target().0;
    let second_target = flows[1].admission_target().0;

    let first = NativeRuntimeAdmissionBroker::admit_next(
        booted,
        pool,
        runtime,
        cpu_runtime,
        memory_pool,
        contract.bytes(),
    )?
    .ok()?;
    if first.agent() != first_target
        || !resident.is_disjoint_from(first)
        || pool.len() + first.identity().owned_frame_count() != initial_pool_len
        || runtime.len() != initial_runtime_len + 1
        || !runtime.contains(ADMISSION_SUPERVISOR)
        || !runtime.contains(first_target)
    {
        return None;
    }

    let cancelled_identity = if prove_cancellation {
        let requester = booted
            .kernel()
            .runtime_admissions()
            .iter()
            .find(|record| record.target == first_target)?
            .requester;
        let failure = NativeAddressSpaceService::admit(
            pool,
            runtime,
            cpu_runtime,
            memory_pool,
            flows[0].verified_image(booted, contract.bytes())?,
            flows[0].admitted_call_context(requester)?,
        )?
        .err()?;
        let identity = failure.identity()?;
        if !failure.proves_rollback(
            NativeAddressSpaceAdmissionStage::RuntimeRegistration,
            first_target,
            identity,
        ) || pool.len() + first.identity().owned_frame_count() != initial_pool_len
            || !pool.owns_zeroed(identity)
            || runtime.len() != initial_runtime_len + 1
            || !runtime.contains(ADMISSION_SUPERVISOR)
            || !runtime.contains(first_target)
        {
            return None;
        }
        serial_write_line("AGENT_KERNEL_NATIVE_ADDRESS_SPACE_RUNTIME_CANCEL_OK");
        Some(identity)
    } else {
        None
    };

    let second = NativeRuntimeAdmissionBroker::admit_next(
        booted,
        pool,
        runtime,
        cpu_runtime,
        memory_pool,
        contract.bytes(),
    )?
    .ok()?;
    if second.agent() != second_target
        || cancelled_identity.is_some_and(|identity| second.identity() != identity)
        || !resident.is_disjoint_from(second)
        || !first.is_disjoint_from(second)
        || pool.len() + first.identity().owned_frame_count() + second.identity().owned_frame_count()
            != initial_pool_len
        || pool.owns(first.identity())
        || pool.owns(second.identity())
        || runtime.len() != initial_runtime_len + 2
        || !runtime.contains(ADMISSION_SUPERVISOR)
        || !runtime.contains(first_target)
        || !runtime.contains(second_target)
        || !PreparedReuseWorkerFlow::batch_queued(booted, flows)
        || !memory_pool.all_available_and_zero()
    {
        return None;
    }
    if let Some(expected) = expected_identities {
        if first.identity() != expected[0] || second.identity() != expected[1] {
            return None;
        }
    }

    serial_write_line("AGENT_KERNEL_NATIVE_ADDRESS_SPACE_ALLOCATED_OK");
    serial_write_line("AGENT_KERNEL_NATIVE_ADDRESS_SPACE_REBUILT_OK");
    serial_write_line("AGENT_KERNEL_NATIVE_ADDRESS_SPACE_RUNTIME_BATCH_OK");
    serial_write_line("AGENT_KERNEL_NATIVE_ADDRESS_SPACE_RUNTIME_CONCURRENCY_OK");
    serial_write_line("AGENT_KERNEL_NATIVE_RUNTIME_ADMISSION_COMMIT_OK");
    Some([first, second])
}
