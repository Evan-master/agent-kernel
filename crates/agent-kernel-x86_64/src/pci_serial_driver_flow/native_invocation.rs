//! One native Driver Invocation admission, execution, and reclamation cycle.

use agent_kernel_core::{DriverCommandId, DriverCommandResult};
use agent_kernel_hal::DriverBackend;
use agent_kernel_x86_64::{agent_call::AgentCallContext, agent_image::VerifiedAgentImage};

use crate::{
    agent_cpu::AgentCpuRuntime,
    agent_memory::{NativeAddressSpaceFramePool, RuntimeMemoryPool},
    boot_agent_images::BootPciSerialDriverImage,
    native_address_space_service::NativeAddressSpaceService,
    native_agent_runtime::NativeAgentRuntime,
    native_driver_executor::{self, DriverRecoveryAuthority, NativeDriverFaultEvidence},
    smp_boot::SmpBootstrap,
    X86BootedKernel,
};

pub(super) struct InvocationExecutionEvidence {
    pub(super) command: DriverCommandId,
    pub(super) result: DriverCommandResult,
    pub(super) dispatches: u8,
    pub(super) quantum_expiries: u8,
    pub(super) restart_generation: u8,
    pub(super) fault: Option<NativeDriverFaultEvidence>,
}

#[allow(clippy::too_many_arguments)]
pub(super) fn execute_and_reclaim<B: DriverBackend>(
    booted: &mut X86BootedKernel,
    runtime: &mut NativeAgentRuntime,
    cpu_runtime: &AgentCpuRuntime,
    memory_pool: &RuntimeMemoryPool,
    address_space_pool: &mut NativeAddressSpaceFramePool,
    smp: &mut SmpBootstrap,
    verified_image: VerifiedAgentImage<'_>,
    context: AgentCallContext,
    contract: BootPciSerialDriverImage,
    recovery_authority: DriverRecoveryAuthority,
    backend: &mut B,
) -> Option<InvocationExecutionEvidence> {
    if !runtime.is_empty() {
        return None;
    }
    let initial_pool_len = address_space_pool.len();
    let native_admission = NativeAddressSpaceService::admit(
        address_space_pool,
        runtime,
        cpu_runtime,
        memory_pool,
        verified_image,
        context,
    )?
    .ok()?;
    if native_admission.agent() != context.agent()
        || runtime.len() != 1
        || !runtime.contains(context.agent())
        || address_space_pool.len() + native_admission.identity().owned_frame_count()
            != initial_pool_len
    {
        return None;
    }

    let execution = native_driver_executor::run(
        booted,
        runtime,
        context.agent(),
        context.driver_invocation()?,
        recovery_authority,
        backend,
    )?;
    let completed = execution.completed();
    if completed.context() != context
        || completed.nonce() != contract.nonce()
        || completed.call_count() != 5
        || completed.operations() != contract.expected_operations()
        || completed.return_offsets() != contract.expected_return_offsets()
        || completed.physical_quantum_generation() != 1
        || !completed.reclamation_log().is_empty()
    {
        return None;
    }
    let evidence = InvocationExecutionEvidence {
        command: execution.command(),
        result: execution.result(),
        dispatches: execution.dispatches(),
        quantum_expiries: execution.quantum_expiries(),
        restart_generation: completed.restart_generation(),
        fault: execution.fault(),
    };

    let identity = native_admission.identity();
    let completed = execution.into_completed();
    let reclamation = completed.prepare_address_space_reclamation(address_space_pool)?;
    if reclamation.identity() != identity {
        return None;
    }
    let quarantined = completed.quarantine_address_space(address_space_pool, reclamation)?;
    let shootdown = smp
        .shootdown_address_space(quarantined.tlb_address_space())
        .ok()?;
    let reclaimed = quarantined.reclaim_after_shootdown(address_space_pool, shootdown)?;
    if !reclaimed.matches(context.agent(), identity)
        || !runtime.is_empty()
        || address_space_pool.len() != initial_pool_len
        || !address_space_pool.all_reclaimed_and_zero()
    {
        return None;
    }
    Some(evidence)
}
