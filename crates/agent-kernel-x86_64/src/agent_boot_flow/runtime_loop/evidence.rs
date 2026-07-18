//! Read-only terminal evidence for the native boot runtime loop.
//!
//! This x86 boot-adapter child binds completed and faulted physical CPU objects
//! to immutable Capsule metadata and public semantic state. It performs no
//! scheduling, page-table mutation, or Agent execution.

mod fault_reclamation;

use agent_kernel_core::EventKind;
use agent_kernel_x86_64::{
    agent_call::AgentCallContext,
    native_runtime::NativeAgentFault,
    user_memory::{
        FIRST_AGENT_RESTART_GENERATION, LAZY_DATA_PROOF_VALUE, SECOND_AGENT_RESTART_GENERATION,
        THIRD_AGENT_RESTART_GENERATION,
    },
};

use crate::{
    agent_cpu::CompletedAgentCpu,
    agent_memory::RuntimeMemoryPool,
    boot_agent_images::{BootAgentImage, BootFaultWorkerImage, BootVerifierImage},
    fault_task_flow::{
        expected_lazy_page_fault, expected_page_fault, PreparedFaultTaskFlow, FAULT_WORKER,
    },
    native_agent_executor::{NativeExecutionReport, NativeVerifyAuthority},
    timer_task_flow::{VerificationSubject, WORKER_A, WORKER_B},
    verifier_task_flow::VERIFIER,
    X86BootedKernel,
};

pub(super) fn worker_evidence_valid(
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
        && completed.restart_generation() == 0
        && completed.lazy_data_byte() == 0
}

pub(super) fn verifier_evidence_valid(
    report: &NativeExecutionReport,
    image: BootVerifierImage,
    context: AgentCallContext,
    subject: VerificationSubject,
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
        && completed.restart_generation() == 0
        && completed.lazy_data_byte() == 0
        && subject.task().raw() == image.target()
        && subject.result() == image.result()
}

pub(super) fn invalid_opcode_evidence_valid(
    booted: &X86BootedKernel,
    memory_pool: &RuntimeMemoryPool,
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
        && fault_reclamation::valid(booted, memory_pool, faulted, flow, image)
        && faulted.physical_quantum_generation() == 1
        && faulted.restart_generation() == 0
        && flow.invalid_opcode_faulted_after_runtime(booted)
        && matches!((fault_event, verifier_continuation), (Some(fault), Some(next)) if fault < next)
}

pub(super) fn general_protection_evidence_valid(
    booted: &X86BootedKernel,
    report: &NativeExecutionReport,
    flow: &PreparedFaultTaskFlow,
    image: BootFaultWorkerImage,
    context: AgentCallContext,
) -> bool {
    let Some(faulted) = report.faulted(FAULT_WORKER) else {
        return false;
    };
    faulted.context() == context
        && faulted.fault() == NativeAgentFault::GeneralProtection { error_code: 0 }
        && faulted.fault_offset() == Some(image.general_protection_offset())
        && !faulted.had_call_progress()
        && faulted.physical_quantum_generation() == 1
        && faulted.restart_generation() == FIRST_AGENT_RESTART_GENERATION
        && flow.general_protection_faulted_after_runtime(booted)
}

pub(super) fn fault_recovery_evidence_valid(
    booted: &X86BootedKernel,
    report: &NativeExecutionReport,
    flow: &PreparedFaultTaskFlow,
    image: BootFaultWorkerImage,
    context: AgentCallContext,
) -> bool {
    let Some(completed) = report.completed(FAULT_WORKER) else {
        return false;
    };
    completed.context() == context
        && completed.nonce() == image.nonce()
        && completed.call_count() == 2
        && completed.address_space_switch_count() == 4
        && completed.operations() == image.expected_operations()
        && completed.return_offsets() == image.expected_return_offsets()
        && completed.physical_quantum_generation() == 1
        && completed.restart_generation() == THIRD_AGENT_RESTART_GENERATION
        && completed.lazy_data_byte() == LAZY_DATA_PROOF_VALUE
        && flow.completed_after_fault_recovery(booted)
}

pub(super) fn lazy_page_fault_evidence_valid(
    booted: &X86BootedKernel,
    report: &NativeExecutionReport,
    flow: &PreparedFaultTaskFlow,
    image: BootFaultWorkerImage,
    context: AgentCallContext,
) -> bool {
    let Some(faulted) = report.faulted(FAULT_WORKER) else {
        return false;
    };
    faulted.context() == context
        && faulted.fault() == expected_lazy_page_fault()
        && faulted.fault_offset() == Some(image.lazy_page_fault_offset())
        && !faulted.had_call_progress()
        && faulted.physical_quantum_generation() == 1
        && faulted.restart_generation() == THIRD_AGENT_RESTART_GENERATION
        && flow.lazy_page_faulted_after_runtime(booted)
}

pub(super) fn page_fault_evidence_valid(
    booted: &X86BootedKernel,
    report: &NativeExecutionReport,
    flow: &PreparedFaultTaskFlow,
    image: BootFaultWorkerImage,
    context: AgentCallContext,
) -> bool {
    let Some(faulted) = report.faulted(FAULT_WORKER) else {
        return false;
    };
    let expected_fault = expected_page_fault();
    faulted.context() == context
        && faulted.fault() == expected_fault
        && faulted.fault_offset() == Some(image.page_fault_offset())
        && !faulted.had_call_progress()
        && faulted.physical_quantum_generation() == 1
        && faulted.restart_generation() == SECOND_AGENT_RESTART_GENERATION
        && flow.page_faulted_after_runtime(booted)
}
