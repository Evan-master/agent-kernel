//! Boot evidence adapter for the generic native Agent runtime loop.
//!
//! This module binds immutable Capsule contracts to terminal transcripts while
//! the executor routes operations independently of Worker or Verifier roles.

use agent_kernel_core::EventKind;
use agent_kernel_x86_64::{agent_call::AgentCallContext, native_runtime::NativeAgentFault};

use crate::{
    agent_cpu::CompletedAgentCpu,
    boot_agent_images::{BootAgentImage, BootFaultWorkerImage, BootVerifierImage},
    fault_task_flow::{PreparedFaultTaskFlow, FAULT_WORKER},
    native_agent_executor::{
        self, NativeExecutionReport, NativeRuntimeEvidence, NativeVerifyAuthority,
    },
    native_agent_runtime::NativeAgentRuntime,
    serial_write_line,
    timer_task_flow::{QueuedTimerTaskFlow, WORKER_A, WORKER_B},
    verifier_task_flow::{PreparedVerifierFlow, VERIFIER},
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
        }
    }
}

pub(super) fn run(
    booted: &mut X86BootedKernel,
    runtime: &mut NativeAgentRuntime,
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
    } = plan;
    let authority = verifier.runtime_authority()?;
    let mut report = NativeExecutionReport::new();
    let mut evidence = NativeRuntimeEvidence::default();

    native_agent_executor::run_until_idle(
        booted,
        runtime,
        &mut report,
        &mut evidence,
        Some(authority),
    )?;
    if runtime.len() != 2
        || report.len() != 2
        || report.faulted_len() != 0
        || !worker_evidence_valid(&report, worker_images, worker_contexts, authority)
    {
        return None;
    }
    let completed_workers = workers.completed_after_runtime(booted)?;
    write_worker_markers();

    fault.queue_for_runtime(booted)?;
    verifier.queue_after_workers_for_runtime(
        booted,
        &completed_workers,
        Some(fault.run_queue_entry()),
    )?;
    native_agent_executor::run_until_idle(
        booted,
        runtime,
        &mut report,
        &mut evidence,
        Some(authority),
    )?;
    if !runtime.is_empty()
        || report.len() != 3
        || report.faulted_len() != 1
        || !evidence.proves_current_boot()
        || !fault_evidence_valid(booted, &report, &fault, fault_image, fault_context)
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
    write_verifier_markers();
    Some(())
}

fn worker_evidence_valid(
    report: &NativeExecutionReport,
    images: [BootAgentImage; 2],
    contexts: [AgentCallContext; 2],
    authority: NativeVerifyAuthority,
) -> bool {
    let Some(first) = report.completed(WORKER_A) else {
        return false;
    };
    let Some(second) = report.completed(WORKER_B) else {
        return false;
    };
    completed_matches_worker(first, images[0], contexts[0], 2)
        && completed_matches_worker(second, images[1], contexts[1], 1)
        && first.nonce() != second.nonce()
        && authority.resolve(first.context().agent()).is_none()
        && authority.resolve(second.context().agent()).is_none()
}

fn completed_matches_worker(
    completed: &CompletedAgentCpu,
    image: BootAgentImage,
    context: AgentCallContext,
    physical_quantum_generation: u8,
) -> bool {
    completed.context() == context
        && completed.nonce() == image.nonce()
        && completed.call_count() == 5
        && completed.address_space_switch_count() == 10
        && completed.operations() == image.expected_operations()
        && completed.return_offsets() == image.expected_return_offsets()
        && completed.physical_quantum_generation() == physical_quantum_generation
}

fn verifier_evidence_valid(
    report: &NativeExecutionReport,
    image: BootVerifierImage,
    context: AgentCallContext,
    subject: crate::timer_task_flow::VerificationSubject,
) -> bool {
    let Some(completed) = report.completed(VERIFIER) else {
        return false;
    };
    completed.context() == context
        && completed.nonce() == image.nonce()
        && completed.call_count() == 4
        && completed.address_space_switch_count() == 8
        && completed.operations() == image.expected_operations()
        && completed.return_offsets() == image.expected_return_offsets()
        && completed.physical_quantum_generation() == 1
        && subject.task().raw() == image.target()
        && subject.result() == image.result()
}

fn fault_evidence_valid(
    booted: &X86BootedKernel,
    report: &NativeExecutionReport,
    flow: &PreparedFaultTaskFlow,
    image: BootFaultWorkerImage,
    context: AgentCallContext,
) -> bool {
    let Some(faulted) = report.faulted(FAULT_WORKER) else {
        return false;
    };
    let fault_event = booted
        .kernel()
        .events()
        .iter()
        .position(|event| event.kind == EventKind::TaskFaulted && event.agent == FAULT_WORKER);
    let verifier_continuation = booted
        .kernel()
        .events()
        .iter()
        .position(|event| event.kind == EventKind::TaskResultInspected);
    faulted.context() == context
        && faulted.fault() == NativeAgentFault::InvalidOpcode
        && faulted.fault_offset() == Some(image.invalid_opcode_offset())
        && !faulted.had_call_progress()
        && faulted.physical_quantum_generation() == 1
        && flow.faulted_after_runtime(booted)
        && matches!((fault_event, verifier_continuation), (Some(fault), Some(next)) if fault < next)
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
    serial_write_line("AGENT_KERNEL_NATIVE_VERIFIER_OK");
    serial_write_line("AGENT_KERNEL_NATIVE_RUNTIME_LOOP_OK");
    serial_write_line("AGENT_KERNEL_NATIVE_RUNTIME_QUANTUM_OK");
}
