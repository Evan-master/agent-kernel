//! Supervisor formatting for fault-specific events.
//!
//! This module belongs to the host-side `agent-supervisor` crate. It keeps
//! task fault, handler install, and route formatting out of the general event
//! formatter while preserving deterministic text output for tests.

use agent_kernel_core::Event;

pub(crate) fn format_task_fault_event(event: &Event, label: &str) -> String {
    let agent = event.agent.raw();
    let resource = event
        .resource
        .map(|resource| resource.raw())
        .unwrap_or_default();
    let task = event.task.map(|task| task.raw()).unwrap_or_default();
    let fault = event.fault.map(|fault| fault.raw()).unwrap_or_default();
    let detail = event.fault_detail.unwrap_or_default();

    format!(
        "event[{}] {} agent={} resource={} task={} fault={} detail={}",
        event.sequence, label, agent, resource, task, fault, detail
    )
}

pub(crate) fn format_fault_handler_event(event: &Event, label: &str) -> String {
    let agent = event.agent.raw();
    let resource = event
        .resource
        .map(|resource| resource.raw())
        .unwrap_or_default();
    let target_agent = event
        .target_agent
        .map(|agent| agent.raw())
        .unwrap_or_default();

    format!(
        "event[{}] {} agent={} resource={} target_agent={}",
        event.sequence, label, agent, resource, target_agent
    )
}

pub(crate) fn format_fault_route_event(event: &Event, label: &str) -> String {
    let agent = event.agent.raw();
    let resource = event
        .resource
        .map(|resource| resource.raw())
        .unwrap_or_default();
    let task = event.task.map(|task| task.raw()).unwrap_or_default();
    let fault = event.fault.map(|fault| fault.raw()).unwrap_or_default();
    let detail = event.fault_detail.unwrap_or_default();
    let target_agent = event
        .target_agent
        .map(|agent| agent.raw())
        .unwrap_or_default();
    let message = event
        .message
        .map(|message| message.raw())
        .unwrap_or_default();

    format!(
        "event[{}] {} agent={} resource={} task={} fault={} detail={} target_agent={} message={}",
        event.sequence, label, agent, resource, task, fault, detail, target_agent, message
    )
}
