//! Two resident Runtime Admission batches over reclaimed address spaces.

mod batch;
mod release;

use agent_kernel_core::RunQueueEntry;
use agent_kernel_x86_64::address_space::AGENT_OWNED_FRAME_COUNT;

use crate::{
    admission_supervisor_flow::{PreparedAdmissionSupervisorFlow, ADMISSION_SUPERVISOR},
    agent_cpu::AgentCpuRuntime,
    agent_memory::{
        NativeAddressSpaceFramePool, RuntimeMemoryPool, NATIVE_ADDRESS_SPACE_FRAME_CAPACITY,
    },
    boot_agent_images::{BootAdmissionSupervisorImage, BootReuseWorkerImage},
    native_address_space_service::NativeAddressSpaceService,
    native_agent_executor::{self, NativeExecutionReport, NativeRuntimeEvidence},
    native_agent_runtime::NativeAgentRuntime,
    reuse_worker_flow::{PreparedReuseWorkerFlow, REUSE_WORKER_BATCHES},
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
        || !booted.kernel().run_queue().is_empty()
        || !memory_pool.all_available_and_zero()
        || !address_space_pool.all_reclaimed_and_zero()
    {
        return None;
    }

    let first_flows = prepare_batch(booted, REUSE_WORKER_BATCHES[0], worker_contract)?;
    if !PreparedReuseWorkerFlow::batch_unqueued(booted, &first_flows)
        || !booted.kernel().run_queue().is_empty()
    {
        return None;
    }

    let supervisor =
        PreparedAdmissionSupervisorFlow::prepare(booted, supervisor_contract.digest())?;
    let second_flows = prepare_batch(booted, REUSE_WORKER_BATCHES[1], worker_contract)?;
    let supervisor_context = supervisor.call_context()?;
    if !PreparedReuseWorkerFlow::batch_unqueued(booted, &second_flows)
        || booted.kernel().run_queue()
            != [RunQueueEntry {
                task: supervisor_context.task(),
                agent: ADMISSION_SUPERVISOR,
            }]
    {
        return None;
    }

    let supervisor_admission = NativeAddressSpaceService::admit(
        address_space_pool,
        runtime,
        cpu_runtime,
        memory_pool,
        supervisor.verified_image(booted, supervisor_contract.bytes())?,
        supervisor_context,
    )?
    .ok()?;
    if supervisor_admission.agent() != ADMISSION_SUPERVISOR
        || address_space_pool.len() + AGENT_OWNED_FRAME_COUNT != NATIVE_ADDRESS_SPACE_FRAME_CAPACITY
        || runtime.len() != 1
        || !runtime.contains(ADMISSION_SUPERVISOR)
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
    let first_targets = [
        first_flows[0].admission_target(),
        first_flows[1].admission_target(),
    ];
    let second_targets = [
        second_flows[0].admission_target(),
        second_flows[1].admission_target(),
    ];
    let all_targets = [
        first_targets[0],
        first_targets[1],
        second_targets[0],
        second_targets[1],
    ];
    if runtime.len() != 1
        || !runtime.contains(ADMISSION_SUPERVISOR)
        || report.len() != 0
        || report.faulted_len() != 0
        || !evidence.proves_runtime_admission_wait()
        || !supervisor.waiting_after_requests(booted, first_targets)
        || address_space_pool.len() + AGENT_OWNED_FRAME_COUNT != NATIVE_ADDRESS_SPACE_FRAME_CAPACITY
        || address_space_pool.owns(supervisor_admission.identity())
        || !memory_pool.all_available_and_zero()
    {
        return None;
    }
    serial_write_line("AGENT_KERNEL_NATIVE_RUNTIME_ADMISSION_REQUEST_OK");
    serial_write_line("AGENT_KERNEL_NATIVE_RUNTIME_ADMISSION_RESIDENT_WAIT_OK");

    let first_admissions = batch::admit(
        booted,
        address_space_pool,
        runtime,
        cpu_runtime,
        memory_pool,
        worker_contract,
        &first_flows,
        supervisor_admission,
        None,
        true,
    )?;
    native_agent_executor::run_until_idle(
        booted,
        runtime,
        memory_pool,
        &mut report,
        &mut evidence,
        None,
    )?;
    if runtime.len() != 1
        || !runtime.contains(ADMISSION_SUPERVISOR)
        || report.len() != 2
        || report.faulted_len() != 0
        || !evidence.proves_resident_runtime_admission_flow()
        || first_flows
            .iter()
            .any(|flow| !flow.completed_after_runtime(booted, &report, worker_contract))
        || !supervisor.waiting_between_batches(booted, all_targets)
        || address_space_pool.len() + 3 * AGENT_OWNED_FRAME_COUNT
            != NATIVE_ADDRESS_SPACE_FRAME_CAPACITY
        || !memory_pool.all_available_and_zero()
    {
        return None;
    }
    serial_write_line("AGENT_KERNEL_NATIVE_RUNTIME_ADMISSION_NOTIFICATION_OK");
    serial_write_line("AGENT_KERNEL_NATIVE_RUNTIME_ADMISSION_REQUEST_OK");
    serial_write_line("AGENT_KERNEL_NATIVE_RUNTIME_ADMISSION_RESIDENT_WAIT_OK");
    serial_write_line("AGENT_KERNEL_NATIVE_ADDRESS_SPACE_REUSE_EXECUTION_OK");

    release::partial(
        booted,
        &mut report,
        address_space_pool,
        runtime,
        memory_pool,
        &supervisor,
        &first_flows,
        supervisor_admission,
        first_admissions,
    )?;

    let second_admissions = batch::admit(
        booted,
        address_space_pool,
        runtime,
        cpu_runtime,
        memory_pool,
        worker_contract,
        &second_flows,
        supervisor_admission,
        Some([
            first_admissions[1].identity(),
            first_admissions[0].identity(),
        ]),
        false,
    )?;
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
        || !evidence.proves_repeated_runtime_admission_flow()
        || second_flows
            .iter()
            .any(|flow| !flow.completed_after_runtime(booted, &report, worker_contract))
        || !supervisor.completed_after_notifications(
            booted,
            &report,
            supervisor_contract,
            all_targets,
        )
        || address_space_pool.len() + 3 * AGENT_OWNED_FRAME_COUNT
            != NATIVE_ADDRESS_SPACE_FRAME_CAPACITY
        || !memory_pool.all_available_and_zero()
    {
        return None;
    }
    serial_write_line("AGENT_KERNEL_NATIVE_RUNTIME_ADMISSION_NOTIFICATION_OK");
    serial_write_line("AGENT_KERNEL_NATIVE_RUNTIME_ADMISSION_SUPERVISOR_OK");
    serial_write_line("AGENT_KERNEL_NATIVE_RUNTIME_ADMISSION_COMPACTION_OK");
    serial_write_line("AGENT_KERNEL_NATIVE_ADDRESS_SPACE_REUSE_EXECUTION_OK");

    release::terminal(
        booted,
        &mut report,
        address_space_pool,
        runtime,
        memory_pool,
        &supervisor,
        &second_flows,
        supervisor_admission,
        second_admissions,
    )
}

fn prepare_batch(
    booted: &mut X86BootedKernel,
    agents: [agent_kernel_core::AgentId; 2],
    contract: BootReuseWorkerImage,
) -> Option<[PreparedReuseWorkerFlow; 2]> {
    Some([
        PreparedReuseWorkerFlow::prepare_unqueued(booted, agents[0], contract)?,
        PreparedReuseWorkerFlow::prepare_unqueued(booted, agents[1], contract)?,
    ])
}
