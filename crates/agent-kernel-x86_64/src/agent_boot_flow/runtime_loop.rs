//! Boot evidence adapter for the generic native Agent runtime loop.
//!
//! This module binds immutable Capsule contracts to terminal transcripts while
//! the executor routes operations independently of Worker or Verifier roles.

mod evidence;

use agent_kernel_x86_64::{agent_call::AgentCallContext, native_runtime::NativeAgentFault};

use self::evidence::{
    fault_recovery_evidence_valid, general_protection_evidence_valid,
    invalid_opcode_evidence_valid, lazy_page_fault_evidence_valid, page_fault_evidence_valid,
    verifier_evidence_valid, worker_evidence_valid,
};

use crate::{
    agent_memory::{NativeAddressSpaceFramePool, RuntimeMemoryPool},
    boot_agent_images::{
        BootAgentImage, BootFaultHandlerImage, BootFaultWorkerImage, BootResourceManagerImage,
        BootVerifierImage,
    },
    fault_handler_flow::PreparedFaultHandlerFlow,
    fault_task_flow::{expected_page_fault, PreparedFaultTaskFlow},
    native_agent_executor::{self, NativeExecutionReport, NativeRuntimeEvidence},
    native_agent_runtime::NativeAgentRuntime,
    resource_manager_flow::PreparedResourceManagerFlow,
    serial_write_line,
    timer_task_flow::QueuedTimerTaskFlow,
    verifier_task_flow::PreparedVerifierFlow,
    X86BootedKernel,
};

pub(super) struct RuntimeLoopPlan {
    workers: QueuedTimerTaskFlow,
    verifier: PreparedVerifierFlow,
    worker_images: [BootAgentImage; 2],
    worker_contexts: [AgentCallContext; 2],
    verifier_image: BootVerifierImage,
    verifier_context: AgentCallContext,
    fault: PreparedFaultTaskFlow,
    fault_image: BootFaultWorkerImage,
    fault_context: AgentCallContext,
    fault_handler: PreparedFaultHandlerFlow,
    fault_handler_image: BootFaultHandlerImage,
    resource_manager: PreparedResourceManagerFlow,
    resource_manager_image: BootResourceManagerImage,
}

impl RuntimeLoopPlan {
    pub(super) const fn new(
        workers: QueuedTimerTaskFlow,
        verifier: PreparedVerifierFlow,
        worker_images: [BootAgentImage; 2],
        worker_contexts: [AgentCallContext; 2],
        verifier_image: BootVerifierImage,
        verifier_context: AgentCallContext,
        fault: PreparedFaultTaskFlow,
        fault_image: BootFaultWorkerImage,
        fault_context: AgentCallContext,
        fault_handler: PreparedFaultHandlerFlow,
        fault_handler_image: BootFaultHandlerImage,
        resource_manager: PreparedResourceManagerFlow,
        resource_manager_image: BootResourceManagerImage,
    ) -> Self {
        Self {
            workers,
            verifier,
            worker_images,
            worker_contexts,
            verifier_image,
            verifier_context,
            fault,
            fault_image,
            fault_context,
            fault_handler,
            fault_handler_image,
            resource_manager,
            resource_manager_image,
        }
    }
}

pub(super) fn run(
    booted: &mut X86BootedKernel,
    runtime: &mut NativeAgentRuntime,
    memory_pool: &mut RuntimeMemoryPool,
    address_space_pool: &mut NativeAddressSpaceFramePool,
    plan: RuntimeLoopPlan,
) -> Option<()> {
    let RuntimeLoopPlan {
        workers,
        verifier,
        worker_images,
        worker_contexts,
        verifier_image,
        verifier_context,
        fault,
        fault_image,
        fault_context,
        fault_handler,
        fault_handler_image,
        resource_manager,
        resource_manager_image,
    } = plan;
    let authority = verifier.runtime_authority()?;
    let mut report = NativeExecutionReport::new();
    let mut evidence = NativeRuntimeEvidence::default();

    native_agent_executor::run_until_idle(
        booted,
        runtime,
        memory_pool,
        &mut report,
        &mut evidence,
        Some(authority),
    )?;
    if runtime.len() != 4
        || report.len() != 2
        || report.faulted_len() != 0
        || !worker_evidence_valid(&report, worker_images, worker_contexts, authority)
    {
        return None;
    }
    let completed_workers = workers.completed_after_runtime(booted)?;
    write_worker_markers();

    fault_handler.queue_for_runtime(booted)?;
    native_agent_executor::run_until_idle(
        booted,
        runtime,
        memory_pool,
        &mut report,
        &mut evidence,
        Some(authority),
    )?;
    if runtime.len() != 4
        || report.len() != 2
        || report.faulted_len() != 0
        || !fault_handler.waiting_after_runtime(booted)
    {
        return None;
    }
    serial_write_line("AGENT_KERNEL_NATIVE_FAULT_HANDLER_WAIT_OK");

    fault.queue_for_runtime(booted)?;
    verifier.queue_after_workers_for_runtime(
        booted,
        &completed_workers,
        Some(fault.run_queue_entry()),
    )?;
    native_agent_executor::run_until_idle(
        booted,
        runtime,
        memory_pool,
        &mut report,
        &mut evidence,
        Some(authority),
    )?;
    if runtime.len() != 2
        || report.len() != 3
        || report.faulted_len() != 1
        || !evidence.proves_fault_containment_phase()
        || !invalid_opcode_evidence_valid(
            booted,
            memory_pool,
            &report,
            &fault,
            fault_image,
            fault_context,
        )
        || !verifier_evidence_valid(
            &report,
            verifier_image,
            verifier_context,
            completed_workers.subject(),
        )
    {
        return None;
    }
    verifier.completed_after_runtime(booted, &completed_workers)?;
    fault.restart_for_runtime(
        booted,
        runtime,
        &mut report,
        NativeAgentFault::InvalidOpcode,
    )?;
    if runtime.len() != 3 || report.faulted_len() != 0 {
        return None;
    }
    native_agent_executor::run_until_idle(
        booted,
        runtime,
        memory_pool,
        &mut report,
        &mut evidence,
        Some(authority),
    )?;
    if runtime.len() != 2
        || report.len() != 3
        || report.faulted_len() != 1
        || !evidence.proves_general_protection_phase()
        || !general_protection_evidence_valid(booted, &report, &fault, fault_image, fault_context)
    {
        return None;
    }
    fault.restart_for_runtime(
        booted,
        runtime,
        &mut report,
        NativeAgentFault::GeneralProtection { error_code: 0 },
    )?;
    if runtime.len() != 3 || report.faulted_len() != 0 {
        return None;
    }
    native_agent_executor::run_until_idle(
        booted,
        runtime,
        memory_pool,
        &mut report,
        &mut evidence,
        Some(authority),
    )?;
    if runtime.len() != 2
        || report.len() != 3
        || report.faulted_len() != 1
        || !evidence.proves_page_fault_phase()
        || !page_fault_evidence_valid(booted, &report, &fault, fault_image, fault_context)
    {
        return None;
    }
    fault.restart_for_runtime(booted, runtime, &mut report, expected_page_fault())?;
    if runtime.len() != 3 || report.faulted_len() != 0 {
        return None;
    }
    native_agent_executor::run_until_idle(
        booted,
        runtime,
        memory_pool,
        &mut report,
        &mut evidence,
        Some(authority),
    )?;
    if runtime.len() != 2
        || report.len() != 3
        || report.faulted_len() != 1
        || !evidence.proves_lazy_page_fault_phase()
        || !lazy_page_fault_evidence_valid(booted, &report, &fault, fault_image, fault_context)
    {
        return None;
    }
    let routed = fault.route_lazy_fault_to_handler(booted)?;
    serial_write_line("AGENT_KERNEL_NATIVE_FAULT_POLICY_ROUTE_OK");
    native_agent_executor::run_until_idle(
        booted,
        runtime,
        memory_pool,
        &mut report,
        &mut evidence,
        Some(authority),
    )?;
    if runtime.len() != 1
        || report.len() != 4
        || report.faulted_len() != 1
        || !evidence.proves_fault_handler_decision_phase()
    {
        return None;
    }
    let approval =
        fault_handler.approve_after_runtime(booted, &report, fault_handler_image, routed)?;
    serial_write_line("AGENT_KERNEL_NATIVE_FAULT_HANDLER_DECISION_OK");
    fault.repair_page_for_runtime(booted, runtime, &mut report, approval)?;
    serial_write_line("AGENT_KERNEL_NATIVE_FAULT_REPAIR_ADMITTED_OK");
    if runtime.len() != 2 || report.faulted_len() != 0 {
        return None;
    }
    serial_write_line("AGENT_KERNEL_NATIVE_FAULT_REPAIR_RUNTIME_OK");
    native_agent_executor::run_until_idle(
        booted,
        runtime,
        memory_pool,
        &mut report,
        &mut evidence,
        Some(authority),
    )?;
    serial_write_line("AGENT_KERNEL_NATIVE_FAULT_RECOVERY_EXECUTION_OK");
    if runtime.len() != 1 || report.len() != 5 || report.faulted_len() != 0 {
        return None;
    }
    serial_write_line("AGENT_KERNEL_NATIVE_FAULT_RECOVERY_COUNTS_OK");
    if !evidence.proves_current_boot() {
        return None;
    }
    serial_write_line("AGENT_KERNEL_NATIVE_FAULT_RECOVERY_COUNTERS_OK");
    if !fault_recovery_evidence_valid(booted, &report, &fault, fault_image, fault_context) {
        return None;
    }
    serial_write_line("AGENT_KERNEL_NATIVE_RESOURCE_MANAGER_READY_OK");
    resource_manager.queue_for_runtime(booted)?;
    serial_write_line("AGENT_KERNEL_NATIVE_RESOURCE_MANAGER_QUEUED_OK");
    if native_agent_executor::run_until_idle(
        booted,
        runtime,
        memory_pool,
        &mut report,
        &mut evidence,
        Some(authority),
    )
    .is_none()
    {
        serial_write_line("AGENT_KERNEL_NATIVE_RESOURCE_MANAGER_EXECUTION_ERROR");
        return None;
    }
    serial_write_line("AGENT_KERNEL_NATIVE_RESOURCE_MANAGER_EXECUTION_OK");
    if !runtime.is_empty()
        || report.len() != 6
        || report.faulted_len() != 0
        || !evidence.proves_resource_manager_phase()
    {
        return None;
    }
    serial_write_line("AGENT_KERNEL_NATIVE_RESOURCE_MANAGER_COUNTERS_OK");
    resource_manager.completed_after_runtime(
        booted,
        &report,
        memory_pool,
        resource_manager_image,
    )?;
    serial_write_line("AGENT_KERNEL_NATIVE_ORPHANED_MESSAGE_RETIREMENT_OK");
    serial_write_line("AGENT_KERNEL_RUNTIME_FRAME_POOL_RELEASED_OK");
    serial_write_line("AGENT_KERNEL_NATIVE_RESOURCE_MANAGER_AGENT_OK");
    serial_write_line("AGENT_KERNEL_NATIVE_CAPABILITY_MANAGER_OK");
    serial_write_line("AGENT_KERNEL_NATIVE_TASK_MANAGER_OK");
    serial_write_line("AGENT_KERNEL_NATIVE_AGENT_MANAGER_OK");
    serial_write_line("AGENT_KERNEL_NATIVE_MEMORY_PAGE_MANAGER_OK");
    serial_write_line("AGENT_KERNEL_NATIVE_MEMORY_REGION_MANAGER_OK");
    serial_write_line("AGENT_KERNEL_NATIVE_MEMORY_CONCURRENCY_OK");
    let completed_agents = [
        worker_contexts[0].agent(),
        worker_contexts[1].agent(),
        verifier_context.agent(),
        fault_context.agent(),
        fault_handler.call_context()?.agent(),
        resource_manager.call_context()?.agent(),
    ];
    report.reclaim_completed_address_spaces(address_space_pool, completed_agents)?;
    serial_write_line("AGENT_KERNEL_NATIVE_ADDRESS_SPACE_RECLAIMED_OK");
    serial_write_line("AGENT_KERNEL_NATIVE_ADDRESS_SPACE_FRAME_POOL_OK");
    write_verifier_markers();
    Some(())
}

fn write_worker_markers() {
    serial_write_line("AGENT_KERNEL_PIT_IRQ_OK");
    serial_write_line("AGENT_KERNEL_AGENT_CPU_PREEMPTION_OK");
    serial_write_line("AGENT_KERNEL_AGENT_RING3_PREEMPTION_OK");
    serial_write_line("AGENT_KERNEL_AGENT_A_PREEMPTION_OK");
    serial_write_line("AGENT_KERNEL_TIMER_PREEMPTION_OK");
    serial_write_line("AGENT_KERNEL_KERNEL_SELECTED_DISPATCH_OK");
    serial_write_line("AGENT_KERNEL_AGENT_CALL_RECEIVE_WAIT_OK");
    serial_write_line("AGENT_KERNEL_NATIVE_BLOCKING_MAILBOX_WAIT_OK");
    serial_write_line("AGENT_KERNEL_AGENT_CALL_SEND_MESSAGE_OK");
    serial_write_line("AGENT_KERNEL_NATIVE_BLOCKING_MAILBOX_WAKE_OK");
    serial_write_line("AGENT_KERNEL_MULTI_AGENT_ISOLATION_OK");
    serial_write_line("AGENT_KERNEL_AGENT_CALL_RECEIVE_MESSAGE_OK");
    serial_write_line("AGENT_KERNEL_AGENT_CALL_ACKNOWLEDGE_MESSAGE_OK");
    serial_write_line("AGENT_KERNEL_AGENT_CPU_RESUME_OK");
    serial_write_line("AGENT_KERNEL_AGENT_CALL_RESULT_OK");
    serial_write_line("AGENT_KERNEL_AGENT_CALL_RETURNING_MUTATION_OK");
    serial_write_line("AGENT_KERNEL_NATIVE_MAILBOX_IPC_OK");
    serial_write_line("AGENT_KERNEL_NATIVE_AGENT_YIELD_OK");
}

fn write_verifier_markers() {
    serial_write_line("AGENT_KERNEL_NATIVE_RUNTIME_STORE_OK");
    serial_write_line("AGENT_KERNEL_VERIFIER_PREEMPTION_OK");
    serial_write_line("AGENT_KERNEL_AGENT_CALL_INSPECT_RESULT_OK");
    serial_write_line("AGENT_KERNEL_AGENT_CALL_VERIFY_OK");
    serial_write_line("AGENT_KERNEL_RESUMABLE_RUNTIME_REGISTRY_OK");
    serial_write_line("AGENT_KERNEL_DISPATCH_READINESS_HANDOFF_OK");
    serial_write_line("AGENT_KERNEL_NATIVE_AGENT_FAULT_CONTAINMENT_OK");
    serial_write_line("AGENT_KERNEL_NATIVE_AGENT_FAULT_RESTART_OK");
    serial_write_line("AGENT_KERNEL_NATIVE_AGENT_GENERAL_PROTECTION_OK");
    serial_write_line("AGENT_KERNEL_NATIVE_AGENT_PAGE_FAULT_OK");
    serial_write_line("AGENT_KERNEL_NATIVE_AGENT_DEMAND_PAGE_OK");
    serial_write_line("AGENT_KERNEL_NATIVE_FAULT_HANDLER_AGENT_OK");
    serial_write_line("AGENT_KERNEL_NATIVE_VERIFIER_OK");
    serial_write_line("AGENT_KERNEL_NATIVE_RUNTIME_LOOP_OK");
    serial_write_line("AGENT_KERNEL_NATIVE_RUNTIME_QUANTUM_OK");
}
