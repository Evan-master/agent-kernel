//! Read-only semantic and physical proof for one Handler approval.

use agent_kernel_core::{
    AgentExecutionState, EventKind, MessageKind, MessageStatus, TaskStatus, WaiterKind,
};

use super::{ApprovedFaultRepair, FaultHandlerTask, FAULT_HANDLER};
use crate::{
    boot_agent_images::BootFaultHandlerImage, fault_task_flow::RoutedFault,
    native_agent_executor::NativeExecutionReport, X86BootedKernel,
};

pub(super) fn waiting(booted: &X86BootedKernel, handler: FaultHandlerTask) -> bool {
    let kernel = booted.kernel();
    let task = kernel.tasks().iter().find(|task| task.id == handler.task);
    let context = kernel
        .execution_contexts()
        .iter()
        .find(|context| context.agent == FAULT_HANDLER);
    let waiter = kernel
        .waiters()
        .iter()
        .find(|waiter| waiter.agent == FAULT_HANDLER && waiter.active);
    matches!(task, Some(task)
        if task.status == TaskStatus::Waiting
            && task.assignee == Some(FAULT_HANDLER)
            && task.delegated_capability == Some(handler.capability)
            && task.run_ticks == 1
            && task.result.is_none())
        && matches!(context, Some(context)
            if context.state == AgentExecutionState::Waiting
                && context.task == Some(handler.task))
        && matches!(waiter, Some(waiter)
            if waiter.kind == WaiterKind::Mailbox && waiter.task == handler.task)
        && !kernel
            .run_queue()
            .iter()
            .any(|entry| entry.agent == FAULT_HANDLER)
        && !kernel.messages().iter().any(|message| {
            message.recipient == FAULT_HANDLER && message.status == MessageStatus::Pending
        })
        && matches!(kernel.events().last(), Some(event)
            if event.kind == EventKind::MessageWaitStarted
                && event.agent == FAULT_HANDLER
                && event.task == Some(handler.task))
}

pub(super) fn approved(
    booted: &X86BootedKernel,
    report: &NativeExecutionReport,
    handler: FaultHandlerTask,
    image: BootFaultHandlerImage,
    routed: RoutedFault,
) -> Option<ApprovedFaultRepair> {
    let kernel = booted.kernel();
    let completed = report.completed(FAULT_HANDLER)?;
    let context = handler.call_context()?;
    let task = kernel.tasks().iter().find(|task| task.id == handler.task)?;
    let fault = kernel
        .faults()
        .iter()
        .find(|fault| fault.id == routed.fault())?;
    let target = kernel.tasks().iter().find(|task| task.id == fault.task)?;
    let message = kernel
        .messages()
        .iter()
        .find(|message| message.id == routed.message())?;
    let expected_result = image.approval();

    if expected_result.value != routed.fault().raw()
        || completed.context() != context
        || completed.nonce() != image.nonce()
        || completed.call_count() != 5
        || completed.address_space_switch_count() != 10
        || completed.operations() != image.expected_operations()
        || completed.return_offsets() != image.expected_return_offsets()
        || completed.physical_quantum_generation() != 1
        || completed.restart_generation() != 0
        || completed.lazy_data_byte() != 0
        || task.status != TaskStatus::Completed
        || task.assignee != Some(FAULT_HANDLER)
        || task.delegated_capability != Some(handler.capability)
        || task.run_ticks != 1
        || task.result != Some(expected_result)
        || target.status != TaskStatus::Faulted
        || target.last_fault != Some(routed.fault())
        || message.sender != booted.report().bootstrap_agent
        || message.recipient != FAULT_HANDLER
        || message.kind != MessageKind::Fault
        || message.status != MessageStatus::Acknowledged
        || message.payload.resource != Some(fault.resource)
        || message.payload.intent != Some(target.intent)
        || message.payload.task != Some(fault.task)
        || message.payload.fault != Some(routed.fault())
        || message.payload.capability.is_some()
        || message.payload.action.is_some()
        || !kernel.run_queue().is_empty()
        || !events_prove_decision(booted, handler, routed)
    {
        return None;
    }

    Some(ApprovedFaultRepair {
        fault: routed.fault(),
    })
}

fn events_prove_decision(
    booted: &X86BootedKernel,
    handler: FaultHandlerTask,
    routed: RoutedFault,
) -> bool {
    let events = booted.kernel().events();
    let expected = [
        EventKind::MessageSent,
        EventKind::MessageWaitWoken,
        EventKind::FaultRouted,
        EventKind::FaultPolicyApplied,
        EventKind::TaskDispatched,
        EventKind::MessageReceived,
        EventKind::MessageAcknowledged,
        EventKind::TaskResultSubmitted,
        EventKind::TaskCompleted,
    ];
    let Some(tail) = events.get(events.len().saturating_sub(expected.len())..) else {
        return false;
    };
    tail.iter().map(|event| event.kind).eq(expected)
        && tail[0].message == Some(routed.message())
        && tail[1].message == Some(routed.message())
        && tail[2].fault == Some(routed.fault())
        && tail[2].target_agent == Some(FAULT_HANDLER)
        && tail[3].fault == Some(routed.fault())
        && tail[4].agent == FAULT_HANDLER
        && tail[4].task == Some(handler.task)
        && tail[5].message == Some(routed.message())
        && tail[6].message == Some(routed.message())
        && tail[7].task_result.is_some()
        && tail[8].task == Some(handler.task)
}
