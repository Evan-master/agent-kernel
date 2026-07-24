//! Two resident Runtime Admission batches over reclaimed address spaces.

mod batch;
mod release;

use crate::{
    admission_supervisor_flow::{PreparedAdmissionSupervisorFlow, ADMISSION_SUPERVISOR},
    agent_cpu::AgentCpuRuntime,
    agent_memory::{NativeAddressSpaceFramePool, RuntimeMemoryPool},
    boot_agent_images::{BootAdmissionSupervisorImage, BootReuseWorkerImage},
    native_address_space_service::NativeAddressSpaceService,
    native_agent_executor::{
        self, NativeEventArchive, NativeExecutionReport, NativeRuntimeEvidence,
    },
    native_agent_runtime::NativeAgentRuntime,
    reuse_worker_flow::{PreparedReuseWorkerFlow, REUSE_WORKER_BATCHES},
    serial_write_line,
    smp_boot::SmpBootstrap,
    NativeDurableSession, X86BootedKernel, X86_FAULT_CAPACITY, X86_RUNTIME_ADMISSION_CAPACITY,
    X86_TASK_CAPACITY,
};
use agent_kernel_core::{
    AgentImageId, AgentImageKind, AgentImageStatus, EventKind, Operation, RunQueueEntry,
};
use agent_kernel_x86_64::agent_image::{AgentImageFormat, AgentImageTrust};

pub(super) fn run(
    booted: &mut X86BootedKernel,
    runtime: &mut NativeAgentRuntime,
    memory_pool: &mut RuntimeMemoryPool,
    address_space_pool: &mut NativeAddressSpaceFramePool,
    smp_bootstrap: &mut SmpBootstrap,
    cpu_runtime: &AgentCpuRuntime,
    worker_contract: BootReuseWorkerImage,
    supervisor_contract: BootAdmissionSupervisorImage,
    mut durable_session: Option<&mut NativeDurableSession<'_>>,
) -> Option<NativeEventArchive> {
    let inventory_frame_count = address_space_pool.inventory_frame_count()?;
    if !runtime.is_empty()
        || !booted.kernel().run_queue().is_empty()
        || !memory_pool.all_available_and_zero()
        || !address_space_pool.all_reclaimed_and_zero()
        || address_space_pool.len() != inventory_frame_count
        || booted.kernel().runtime_admission_capacity() != X86_RUNTIME_ADMISSION_CAPACITY
        || X86_RUNTIME_ADMISSION_CAPACITY <= X86_TASK_CAPACITY
        || X86_FAULT_CAPACITY != 4
        || booted.kernel().faults().len() != X86_FAULT_CAPACITY
    {
        return None;
    }
    serial_write_line("AGENT_KERNEL_RUNTIME_ADMISSION_CAPACITY_OK");
    serial_write_line("AGENT_KERNEL_NATIVE_FAULT_STORE_FULL_OK");

    let first_flows = prepare_batch(booted, REUSE_WORKER_BATCHES[0], worker_contract)?;
    if !PreparedReuseWorkerFlow::batch_unqueued(booted, &first_flows)
        || !booted.kernel().run_queue().is_empty()
    {
        return None;
    }
    let rotated_image = first_flows[0].verified_image(booted, worker_contract.bytes())?;
    if rotated_image.format() != AgentImageFormat::SignedPackageV3
        || rotated_image.signer_id() != Some(worker_contract.signer_id())
        || rotated_image.trust() != AgentImageTrust::Signed(worker_contract.signer_id())
    {
        return None;
    }
    serial_write_line("AGENT_KERNEL_NATIVE_ROTATED_SIGNER_ADMISSION_OK");

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
        || address_space_pool.len() + supervisor_admission.identity().owned_frame_count()
            != inventory_frame_count
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
        durable_session.as_deref_mut(),
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
        || address_space_pool.len() + supervisor_admission.identity().owned_frame_count()
            != inventory_frame_count
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
        durable_session.as_deref_mut(),
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
        || address_space_pool.len()
            + supervisor_admission.identity().owned_frame_count()
            + first_admissions[0].identity().owned_frame_count()
            + first_admissions[1].identity().owned_frame_count()
            != inventory_frame_count
        || !memory_pool.all_available_and_zero()
    {
        return None;
    }
    serial_write_line("AGENT_KERNEL_NATIVE_RUNTIME_ADMISSION_NOTIFICATION_OK");
    serial_write_line("AGENT_KERNEL_NATIVE_RUNTIME_ADMISSION_REQUEST_OK");
    serial_write_line("AGENT_KERNEL_NATIVE_RUNTIME_ADMISSION_RESIDENT_WAIT_OK");
    serial_write_line("AGENT_KERNEL_NATIVE_ADDRESS_SPACE_REUSE_EXECUTION_OK");
    serial_write_line("AGENT_KERNEL_NATIVE_WAITER_SLOT_REUSE_OK");

    release::partial(
        booted,
        &mut report,
        address_space_pool,
        smp_bootstrap,
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
        durable_session.as_deref_mut(),
    )?;
    let repeated_flow_valid = evidence.proves_repeated_runtime_admission_flow();
    let workers_completed = second_flows
        .iter()
        .all(|flow| flow.completed_after_runtime(booted, &report, worker_contract));
    let supervisor_completed =
        supervisor.completed_after_notifications(booted, &report, supervisor_contract, all_targets);
    let frame_accounting_valid = address_space_pool.len()
        + supervisor_admission.identity().owned_frame_count()
        + second_admissions[0].identity().owned_frame_count()
        + second_admissions[1].identity().owned_frame_count()
        == inventory_frame_count;
    let memory_pool_valid = memory_pool.all_available_and_zero();
    if !runtime.is_empty()
        || report.len() != 3
        || report.faulted_len() != 0
        || !repeated_flow_valid
        || !workers_completed
        || !supervisor_completed
        || !frame_accounting_valid
        || !memory_pool_valid
    {
        if !runtime.is_empty() {
            serial_write_line("AGENT_KERNEL_NATIVE_REPEAT_RUNTIME_STATE_ERROR");
        } else if report.len() != 3 || report.faulted_len() != 0 {
            serial_write_line("AGENT_KERNEL_NATIVE_REPEAT_REPORT_ERROR");
        } else if !repeated_flow_valid {
            serial_write_line("AGENT_KERNEL_NATIVE_REPEAT_COUNTER_ERROR");
        } else if !workers_completed {
            serial_write_line("AGENT_KERNEL_NATIVE_REPEAT_WORKER_EVIDENCE_ERROR");
        } else if !supervisor_completed {
            serial_write_line("AGENT_KERNEL_NATIVE_REPEAT_SUPERVISOR_EVIDENCE_ERROR");
        } else if !frame_accounting_valid {
            serial_write_line("AGENT_KERNEL_NATIVE_REPEAT_FRAME_ACCOUNTING_ERROR");
        } else {
            serial_write_line("AGENT_KERNEL_NATIVE_REPEAT_MEMORY_POOL_ERROR");
        }
        return None;
    }
    serial_write_line("AGENT_KERNEL_NATIVE_RUNTIME_ADMISSION_NOTIFICATION_OK");
    serial_write_line("AGENT_KERNEL_NATIVE_MESSAGE_RETIREMENT_OK");
    serial_write_line("AGENT_KERNEL_NATIVE_RUNTIME_ADMISSION_SUPERVISOR_OK");
    serial_write_line("AGENT_KERNEL_NATIVE_RUNTIME_ADMISSION_COMPACTION_OK");
    serial_write_line("AGENT_KERNEL_NATIVE_TASK_COMPACTION_OK");
    serial_write_line("AGENT_KERNEL_NATIVE_FAULT_COMPACTION_OK");
    serial_write_line("AGENT_KERNEL_NATIVE_INTENT_COMPACTION_OK");
    serial_write_line("AGENT_KERNEL_NATIVE_AGENT_ENTRY_RETIREMENT_OK");
    serial_write_line("AGENT_KERNEL_NATIVE_CAPABILITY_CLEANUP_REVOCATION_OK");
    serial_write_line("AGENT_KERNEL_NATIVE_CAPABILITY_COMPACTION_OK");
    serial_write_line("AGENT_KERNEL_NATIVE_RESOURCE_RECORD_RETIREMENT_OK");
    serial_write_line("AGENT_KERNEL_NATIVE_RESOURCE_STORE_REUSE_OK");
    serial_write_line("AGENT_KERNEL_NATIVE_MEMORY_CELL_RECORD_RETIREMENT_OK");
    serial_write_line("AGENT_KERNEL_NATIVE_MEMORY_CELL_STORE_REUSE_OK");
    serial_write_line("AGENT_KERNEL_NATIVE_WAITER_COMPACTION_OK");
    serial_write_line("AGENT_KERNEL_NATIVE_EVENT_LOG_FULL_OK");
    serial_write_line("AGENT_KERNEL_NATIVE_EVENT_ARCHIVE_OK");
    serial_write_line("AGENT_KERNEL_NATIVE_ADDRESS_SPACE_REUSE_EXECUTION_OK");

    release::terminal(
        booted,
        &mut report,
        address_space_pool,
        smp_bootstrap,
        runtime,
        memory_pool,
        &supervisor,
        &second_flows,
        supervisor_admission,
        second_admissions,
    )?;
    prove_agent_image_slot_reuse(booted, worker_contract)?;
    Some(report.into_event_archive())
}

fn prove_agent_image_slot_reuse(
    booted: &mut X86BootedKernel,
    contract: BootReuseWorkerImage,
) -> Option<()> {
    let report = *booted.report();
    let event_start = booted.kernel().events().len();
    let fresh = booted
        .kernel_mut()
        .sys_register_agent_image(
            report.bootstrap_agent,
            report.bootstrap_capability,
            report.bootstrap_resource,
            AgentImageKind::Worker,
            contract.digest(),
            1,
            1,
        )
        .ok()?;
    let kernel = booted.kernel();
    let record = kernel.agent_image(fresh).ok()?;
    let event = kernel.events().get(event_start)?;
    if fresh != AgentImageId::new(15)
        || kernel.agent_images().len() != 14
        || !kernel
            .agent_images()
            .iter()
            .enumerate()
            .all(|(index, record)| {
                record.id.raw()
                    == if index < 8 {
                        index as u64 + 1
                    } else {
                        index as u64 + 2
                    }
            })
        || record.owner != report.bootstrap_agent
        || record.resource != report.bootstrap_resource
        || record.kind != AgentImageKind::Worker
        || record.digest != contract.digest()
        || record.abi_version != 1
        || record.entry_version != 1
        || record.status != AgentImageStatus::Pending
        || kernel.events().len() != event_start + 1
        || event.kind != EventKind::AgentImageRegistered
        || event.agent != report.bootstrap_agent
        || event.target_agent != Some(report.bootstrap_agent)
        || event.resource != Some(report.bootstrap_resource)
        || event.capability != Some(report.bootstrap_capability)
        || event.operation != Some(Operation::Act)
        || event.agent_image != Some(fresh)
    {
        return None;
    }
    serial_write_line("AGENT_KERNEL_NATIVE_AGENT_IMAGE_SLOT_REUSE_OK");
    Some(())
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
