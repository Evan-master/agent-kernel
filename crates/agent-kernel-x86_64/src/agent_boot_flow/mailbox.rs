//! Physical x86 execution proof for one native two-Worker mailbox exchange.
//!
//! This architecture boot child composes CPU and semantic type states, checks
//! immutable Capsule evidence, and terminates on any scheduler or IPC mismatch.

use crate::{
    agent_cpu::PreemptedAgentCpu,
    boot_agent_images::BootAgentImage,
    fatal_boot,
    native_agent_runtime::NativeAgentRuntime,
    serial_write_line,
    timer_task_flow::{CompletedWorkerTasks, SecondResumedFlow, WORKER_B},
    X86BootedKernel,
};

pub(super) fn run(
    booted: &mut X86BootedKernel,
    runtime: &mut NativeAgentRuntime,
    second_resumed_flow: SecondResumedFlow,
    preempted_b: PreemptedAgentCpu,
    worker_a: BootAgentImage,
    worker_b: BootAgentImage,
) -> CompletedWorkerTasks {
    if !preempted_b.signal_is_clear() || runtime.len() != 2 {
        fatal_boot("AGENT_KERNEL_MULTI_AGENT_ISOLATION_ERROR");
    }
    let Some(expected_offsets_a) = worker_a.expected_sender_return_offsets() else {
        fatal_boot("AGENT_KERNEL_AGENT_IMAGE_OFFSET_ERROR");
    };
    let Some(expected_offsets_b) = worker_b.expected_receiver_return_offsets() else {
        fatal_boot("AGENT_KERNEL_AGENT_IMAGE_OFFSET_ERROR");
    };
    let Some(requested_receive) = preempted_b.resume_until_message_receive() else {
        fatal_boot("AGENT_KERNEL_AGENT_CPU_RESUME_ERROR");
    };
    if requested_receive.call_count() != 2
        || requested_receive.address_space_switch_count() != 4
        || requested_receive.receive_return_offset() != expected_offsets_b[1]
    {
        fatal_boot("AGENT_KERNEL_AGENT_CALL_RECEIVE_MESSAGE_ERROR");
    }
    let Some((second_waiting_flow, waiting_receive)) =
        second_resumed_flow.wait_for_first(booted, requested_receive)
    else {
        fatal_boot("AGENT_KERNEL_AGENT_CALL_RECEIVE_WAIT_ERROR");
    };
    if waiting_receive.waiter().raw() != 1
        || waiting_receive.receive_return_offset() != expected_offsets_b[1]
        || !waiting_receive.agent_call_is_released()
    {
        fatal_boot("AGENT_KERNEL_AGENT_CALL_RECEIVE_WAIT_ERROR");
    }
    if runtime.park_waiting_mailbox(waiting_receive).is_some() || runtime.len() != 3 {
        fatal_boot("AGENT_KERNEL_NATIVE_RUNTIME_STORE_ERROR");
    }
    serial_write_line("AGENT_KERNEL_AGENT_CALL_RECEIVE_WAIT_OK");
    serial_write_line("AGENT_KERNEL_NATIVE_BLOCKING_MAILBOX_WAIT_OK");
    let Some((first_resumed_flow, dispatched_a)) =
        second_waiting_flow.dispatch_first(booted, runtime)
    else {
        fatal_boot("AGENT_KERNEL_TIMER_PREEMPTION_ERROR");
    };
    let Some(preempted_a) = runtime.take_preempted(dispatched_a) else {
        fatal_boot("AGENT_KERNEL_NATIVE_RUNTIME_STORE_ERROR");
    };
    if !preempted_a.signal_is_clear() || runtime.len() != 2 {
        fatal_boot("AGENT_KERNEL_MULTI_AGENT_ISOLATION_ERROR");
    }
    let Some(requested_a) = preempted_a.resume_until_sender_result() else {
        fatal_boot("AGENT_KERNEL_AGENT_CPU_RESUME_ERROR");
    };
    let nonce_a = requested_a.nonce();
    if requested_a.call_count() != 2
        || requested_a.address_space_switch_count() != 4
        || nonce_a != worker_a.nonce()
        || requested_a.result() != worker_a.result()
        || requested_a.describe_return_offset() != expected_offsets_a[0]
        || requested_a.result_return_offset() != expected_offsets_a[1]
    {
        fatal_boot("AGENT_KERNEL_AGENT_CR3_SWITCH_ERROR");
    }
    let Some((first_result_flow, acknowledged_a)) =
        first_resumed_flow.submit_first_result(booted, requested_a)
    else {
        fatal_boot("AGENT_KERNEL_AGENT_CALL_RESULT_ERROR");
    };
    let Some(requested_send) = acknowledged_a.resume_until_message_send(WORKER_B) else {
        fatal_boot("AGENT_KERNEL_AGENT_CPU_RESUME_ERROR");
    };
    if requested_send.call_count() != 3
        || requested_send.address_space_switch_count() != 6
        || requested_send.send_return_offset() != expected_offsets_a[2]
    {
        fatal_boot("AGENT_KERNEL_AGENT_CALL_SEND_MESSAGE_ERROR");
    }
    let Some((first_message_flow, acknowledged_send)) =
        first_result_flow.send_to_second(booted, requested_send)
    else {
        fatal_boot("AGENT_KERNEL_AGENT_CALL_SEND_MESSAGE_ERROR");
    };
    serial_write_line("AGENT_KERNEL_AGENT_CALL_SEND_MESSAGE_OK");
    serial_write_line("AGENT_KERNEL_NATIVE_BLOCKING_MAILBOX_WAKE_OK");
    let Some(completed_a) = acknowledged_send.resume_until_completion() else {
        fatal_boot("AGENT_KERNEL_AGENT_CPU_RESUME_ERROR");
    };
    if completed_a.call_count() != 4
        || completed_a.address_space_switch_count() != 8
        || completed_a.nonce() != nonce_a
        || completed_a.result() != worker_a.result()
        || completed_a.recipient() != WORKER_B
        || completed_a.message().raw() != 1
        || completed_a.return_offsets() != expected_offsets_a
    {
        fatal_boot("AGENT_KERNEL_AGENT_CR3_SWITCH_ERROR");
    }
    serial_write_line("AGENT_KERNEL_MULTI_AGENT_ISOLATION_OK");
    let Some((second_redispatched_flow, dispatched_b)) =
        first_message_flow.complete_first_and_dispatch_second(booted, completed_a, runtime)
    else {
        fatal_boot("AGENT_KERNEL_AGENT_CPU_COMPLETION_ERROR");
    };
    let Some(waiting_receive) = runtime.take_waiting_mailbox(dispatched_b) else {
        fatal_boot("AGENT_KERNEL_NATIVE_RUNTIME_STORE_ERROR");
    };
    if runtime.len() != 1 {
        fatal_boot("AGENT_KERNEL_NATIVE_RUNTIME_STORE_ERROR");
    }
    let Some((second_message_flow, acknowledged_receive)) =
        second_redispatched_flow.receive_from_first(booted, waiting_receive)
    else {
        fatal_boot("AGENT_KERNEL_AGENT_CALL_RECEIVE_MESSAGE_ERROR");
    };
    serial_write_line("AGENT_KERNEL_AGENT_CALL_RECEIVE_MESSAGE_OK");
    let Some(requested_acknowledgement) =
        acknowledged_receive.resume_until_message_acknowledgement()
    else {
        fatal_boot("AGENT_KERNEL_AGENT_CPU_RESUME_ERROR");
    };
    if requested_acknowledgement.call_count() != 3
        || requested_acknowledgement.address_space_switch_count() != 6
        || requested_acknowledgement.message().raw() != 1
    {
        fatal_boot("AGENT_KERNEL_AGENT_CALL_ACKNOWLEDGE_MESSAGE_ERROR");
    }
    let Some((second_acknowledged_flow, acknowledged_message)) =
        second_message_flow.acknowledge_from_first(booted, requested_acknowledgement)
    else {
        fatal_boot("AGENT_KERNEL_AGENT_CALL_ACKNOWLEDGE_MESSAGE_ERROR");
    };
    serial_write_line("AGENT_KERNEL_AGENT_CALL_ACKNOWLEDGE_MESSAGE_OK");
    let Some(requested_b) = acknowledged_message.resume_until_receiver_result() else {
        fatal_boot("AGENT_KERNEL_AGENT_CPU_RESUME_ERROR");
    };
    if requested_b.call_count() != 4
        || requested_b.address_space_switch_count() != 8
        || requested_b.result() != worker_b.result()
    {
        fatal_boot("AGENT_KERNEL_AGENT_CR3_SWITCH_ERROR");
    }
    let Some((second_result_flow, acknowledged_b)) =
        second_acknowledged_flow.submit_second_result(booted, requested_b)
    else {
        fatal_boot("AGENT_KERNEL_AGENT_CALL_RESULT_ERROR");
    };
    let Some(completed_b) = acknowledged_b.resume_until_completion() else {
        fatal_boot("AGENT_KERNEL_AGENT_CPU_RESUME_ERROR");
    };
    if completed_b.call_count() != 5
        || completed_b.address_space_switch_count() != 10
        || completed_b.nonce() != worker_b.nonce()
        || completed_b.nonce() == nonce_a
        || completed_b.result() != worker_b.result()
        || completed_b.message().id.raw() != 1
        || completed_b.return_offsets() != expected_offsets_b
        || expected_offsets_a[0] == expected_offsets_b[0]
    {
        fatal_boot("AGENT_KERNEL_AGENT_CPU_COMPLETION_ERROR");
    }
    let Some(completed_workers) = second_result_flow.record_second_completion(booted, completed_b)
    else {
        fatal_boot("AGENT_KERNEL_AGENT_CPU_COMPLETION_ERROR");
    };
    if !completed_workers.mailbox_acknowledged(booted) {
        fatal_boot("AGENT_KERNEL_NATIVE_MAILBOX_IPC_ERROR");
    }
    serial_write_line("AGENT_KERNEL_AGENT_CPU_RESUME_OK");
    serial_write_line("AGENT_KERNEL_AGENT_CALL_RESULT_OK");
    serial_write_line("AGENT_KERNEL_AGENT_CALL_RETURNING_MUTATION_OK");
    serial_write_line("AGENT_KERNEL_NATIVE_MAILBOX_IPC_OK");
    completed_workers
}
