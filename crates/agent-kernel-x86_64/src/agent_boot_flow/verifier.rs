//! Physical four-call execution of the scheduled native Verifier.

use crate::{
    agent_cpu::PreparedAgentCpu,
    boot_agent_images::BootVerifierImage,
    serial_write_line,
    timer_task_flow::CompletedWorkerTasks,
    verifier_task_flow::{CompletedVerifierFlow, PreparedVerifierFlow},
    X86BootedKernel,
};

pub(super) fn run(
    booted: &mut X86BootedKernel,
    flow: PreparedVerifierFlow,
    cpu: PreparedAgentCpu,
    image: BootVerifierImage,
    workers: CompletedWorkerTasks,
) -> Option<CompletedVerifierFlow> {
    let subject = workers.subject();
    if subject.task().raw() != image.target() || subject.result() != image.result() {
        return None;
    }
    let running = flow.dispatch_after_workers(booted, workers)?;
    let preempted = cpu.run_until_preempted()?;
    serial_write_line("AGENT_KERNEL_VERIFIER_PREEMPTION_OK");
    let resumed = running.expire_and_redispatch(booted, &preempted)?;

    let requested_inspection = preempted.resume_until_task_inspection(subject.task())?;
    let offsets = image.expected_return_offsets();
    if requested_inspection.call_count() != 2
        || requested_inspection.address_space_switch_count() != 4
        || requested_inspection.nonce() != image.nonce()
        || requested_inspection.target() != subject.task()
        || requested_inspection.describe_return_offset() != offsets[0]
        || requested_inspection.inspection_return_offset() != offsets[1]
    {
        return None;
    }
    let (inspected, acknowledged_inspection) =
        resumed.inspect_subject(booted, requested_inspection)?;
    serial_write_line("AGENT_KERNEL_AGENT_CALL_INSPECT_RESULT_OK");

    let requested_verification = acknowledged_inspection.resume_until_task_verification()?;
    if requested_verification.call_count() != 3
        || requested_verification.address_space_switch_count() != 6
        || requested_verification.target() != subject.task()
        || requested_verification.result() != Some(subject.result())
        || requested_verification.verification_return_offset() != offsets[2]
    {
        return None;
    }
    let (verified, acknowledged_verification) =
        inspected.verify_subject(booted, requested_verification)?;
    serial_write_line("AGENT_KERNEL_AGENT_CALL_VERIFY_OK");

    let completed = acknowledged_verification.resume_until_completion()?;
    if completed.call_count() != 4
        || completed.address_space_switch_count() != 8
        || completed.nonce() != image.nonce()
        || completed.target() != subject.task()
        || completed.result() != subject.result()
        || completed.describe_return_offset() != offsets[0]
        || completed.inspection_return_offset() != offsets[1]
        || completed.verification_return_offset() != offsets[2]
        || completed.completion_return_offset() != offsets[3]
    {
        return None;
    }
    let terminal = verified.complete(booted, completed)?;
    serial_write_line("AGENT_KERNEL_NATIVE_VERIFIER_OK");
    Some(terminal)
}
