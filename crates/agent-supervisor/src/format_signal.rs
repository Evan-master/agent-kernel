//! Supervisor formatting for wait signal events.
//!
//! This module belongs to the host-side `agent-supervisor` crate. It keeps
//! task wait and signal wakeup formatting out of the general event formatter
//! while preserving deterministic text output for tests.

use agent_kernel_core::{Event, WaiterKind};

pub(crate) fn format_task_signal_event(event: &Event, label: &str) -> String {
    let agent = event.agent.raw();
    let resource = event
        .resource
        .map(|resource| resource.raw())
        .unwrap_or_default();
    let task = event.task.map(|task| task.raw()).unwrap_or_default();
    let waiter = event.waiter.map(|waiter| waiter.raw()).unwrap_or_default();
    let signal = event.signal.map(|signal| signal.raw()).unwrap_or_default();

    format!(
        "event[{}] {} agent={} resource={} task={} waiter={} signal={}",
        event.sequence, label, agent, resource, task, waiter, signal
    )
}

pub(crate) fn format_signal_event(event: &Event, label: &str) -> String {
    let agent = event.agent.raw();
    let resource = event
        .resource
        .map(|resource| resource.raw())
        .unwrap_or_default();
    let task = event.task.map(|task| task.raw()).unwrap_or_default();
    let waiter = event.waiter.map(|waiter| waiter.raw()).unwrap_or_default();
    let signal = event.signal.map(|signal| signal.raw()).unwrap_or_default();
    let target_agent = event
        .target_agent
        .map(|agent| agent.raw())
        .unwrap_or_default();

    format!(
        "event[{}] {} agent={} resource={} task={} waiter={} signal={} target_agent={}",
        event.sequence, label, agent, resource, task, waiter, signal, target_agent
    )
}

pub(crate) fn format_mailbox_wait_started_event(event: &Event, label: &str) -> String {
    let agent = event.agent.raw();
    let resource = event
        .resource
        .map(|resource| resource.raw())
        .unwrap_or_default();
    let task = event.task.map(|task| task.raw()).unwrap_or_default();
    let waiter = event.waiter.map(|waiter| waiter.raw()).unwrap_or_default();

    format!(
        "event[{}] {} agent={} resource={} task={} waiter={}",
        event.sequence, label, agent, resource, task, waiter
    )
}

pub(crate) fn format_mailbox_wait_woken_event(event: &Event, label: &str) -> String {
    let target_agent = event
        .target_agent
        .map(|agent| agent.raw())
        .unwrap_or_default();
    let message = event
        .message
        .map(|message| message.raw())
        .unwrap_or_default();

    format!(
        "{} target_agent={} message={}",
        format_mailbox_wait_started_event(event, label),
        target_agent,
        message
    )
}

pub(crate) fn format_waiter_compaction_event(event: &Event) -> String {
    let target_agent = event
        .target_agent
        .map(|agent| agent.raw())
        .unwrap_or_default();
    let resource = event
        .resource
        .map(|resource| resource.raw())
        .unwrap_or_default();
    let authority = event
        .capability
        .map(|capability| capability.raw())
        .unwrap_or_default();
    let task = event.task.map(|task| task.raw()).unwrap_or_default();
    let waiter = event.waiter.map(|waiter| waiter.raw()).unwrap_or_default();
    let signal = event.signal.map(|signal| signal.raw()).unwrap_or_default();
    let kind = event
        .waiter_kind
        .map(waiter_kind_label)
        .unwrap_or("unknown");

    format!(
        "event[{}] waiter_compacted actor={} target_agent={} resource={} authority={} task={} waiter={} kind={} signal={}",
        event.sequence,
        event.agent.raw(),
        target_agent,
        resource,
        authority,
        task,
        waiter,
        kind,
        signal
    )
}

fn waiter_kind_label(kind: WaiterKind) -> &'static str {
    match kind {
        WaiterKind::Signal => "signal",
        WaiterKind::Mailbox => "mailbox",
    }
}
