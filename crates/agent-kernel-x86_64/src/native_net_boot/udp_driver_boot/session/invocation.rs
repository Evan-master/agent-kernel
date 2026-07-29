//! One fresh-address-space Network Driver Invocation.

use agent_kernel_core::{
    DeviceEventId, DeviceEventKind, DeviceEventPayload, DriverCommandId, DriverCommandResult,
    DriverInvocationId, ResourceCreateOutcome,
};
use agent_kernel_hal::DriverBackend;
use agent_kernel_x86_64::agent_call::AgentCallContext;

use crate::{
    agent_cpu::AgentCpuRuntime,
    agent_memory::PreparedAgentMemory,
    boot_agent_images::BootNetworkDriverImage,
    native_agent_runtime::NativeAgentRuntime,
    native_driver_executor::{self, DriverRecoveryAuthority},
    X86BootedKernel,
};

use super::super::{admission::NetworkDriverAdmission, DRIVER};

pub(super) struct InvocationEvidence {
    pub(super) event: DeviceEventId,
    pub(super) invocation: DriverInvocationId,
    pub(super) command: DriverCommandId,
    pub(super) result: DriverCommandResult,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub(super) enum InvocationError {
    RuntimeBusy,
    Event,
    Delivery,
    Context,
    Cpu,
    RuntimeRegistration,
    Executor,
    Contract,
}

impl InvocationError {
    pub(super) const fn diagnostic_marker(self) -> &'static str {
        match self {
            Self::RuntimeBusy => "AGENT_KERNEL_NATIVE_UDP_INVOCATION_RUNTIME_BUSY_ERROR",
            Self::Event => "AGENT_KERNEL_NATIVE_UDP_INVOCATION_EVENT_ERROR",
            Self::Delivery => "AGENT_KERNEL_NATIVE_UDP_INVOCATION_DELIVERY_ERROR",
            Self::Context => "AGENT_KERNEL_NATIVE_UDP_INVOCATION_CONTEXT_ERROR",
            Self::Cpu => "AGENT_KERNEL_NATIVE_UDP_INVOCATION_CPU_ERROR",
            Self::RuntimeRegistration => "AGENT_KERNEL_NATIVE_UDP_INVOCATION_REGISTRATION_ERROR",
            Self::Executor => "AGENT_KERNEL_NATIVE_UDP_INVOCATION_EXECUTOR_ERROR",
            Self::Contract => "AGENT_KERNEL_NATIVE_UDP_INVOCATION_CONTRACT_ERROR",
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) fn run<B: DriverBackend>(
    booted: &mut X86BootedKernel,
    runtime: &mut NativeAgentRuntime,
    cpu_runtime: AgentCpuRuntime,
    memory: PreparedAgentMemory,
    device: ResourceCreateOutcome,
    driver: NetworkDriverAdmission,
    contract: BootNetworkDriverImage,
    recovery_authority: DriverRecoveryAuthority,
    payload: DeviceEventPayload,
    backend: &mut B,
) -> Result<InvocationEvidence, InvocationError> {
    if !runtime.is_empty() {
        return Err(InvocationError::RuntimeBusy);
    }
    let report = *booted.report();
    let event = booted
        .kernel_mut()
        .sys_raise_device_event(
            report.bootstrap_agent,
            device.capability,
            device.resource,
            DeviceEventKind::StateChanged,
            payload,
        )
        .map_err(|_| InvocationError::Event)?;
    let invocation = booted
        .kernel_mut()
        .sys_deliver_device_event(DRIVER, driver.capability, event)
        .map_err(|_| InvocationError::Delivery)?;
    let context = AgentCallContext::new_driver(DRIVER, invocation, driver.image, driver.capability)
        .ok_or(InvocationError::Context)?;
    let cpu = cpu_runtime
        .prepare(memory, context)
        .ok_or(InvocationError::Cpu)?;
    if runtime.register_prepared(cpu).is_some() {
        return Err(InvocationError::RuntimeRegistration);
    }
    let execution = native_driver_executor::run(
        booted,
        runtime,
        DRIVER,
        invocation,
        recovery_authority,
        backend,
    )
    .ok_or(InvocationError::Executor)?;
    let completed = execution.completed();
    if completed.context() != context
        || completed.nonce() != contract.nonce()
        || completed.call_count() != 5
        || completed.operations() != contract.expected_operations()
        || completed.return_offsets() != contract.expected_return_offsets()
        || completed.physical_quantum_generation() != 1
        || completed.restart_generation() != 0
        || execution.dispatches() != 2
        || execution.quantum_expiries() != 1
        || execution.fault().is_some()
        || !runtime.is_empty()
    {
        return Err(InvocationError::Contract);
    }
    Ok(InvocationEvidence {
        event,
        invocation,
        command: execution.command(),
        result: execution.result(),
    })
}
