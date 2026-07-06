//! Supervisor formatting for fault-specific events.
//!
//! This module belongs to the host-side `agent-supervisor` crate. It keeps
//! task fault, handler install, and route formatting out of the general event
//! formatter while preserving deterministic text output for tests.

use agent_kernel_core::Event;

use agent_kernel_core::FaultPolicyAction;

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

pub(crate) fn format_fault_policy_install_event(event: &Event, label: &str) -> String {
    let agent = event.agent.raw();
    let resource = event
        .resource
        .map(|resource| resource.raw())
        .unwrap_or_default();
    let policy = event
        .fault_policy
        .map(|policy| policy.raw())
        .unwrap_or_default();
    let action = format_fault_policy_action(event.fault_policy_action);

    format!(
        "event[{}] {} agent={} resource={} policy={} action={}",
        event.sequence, label, agent, resource, policy, action
    )
}

pub(crate) fn format_fault_policy_apply_event(event: &Event, label: &str) -> String {
    let agent = event.agent.raw();
    let resource = event
        .resource
        .map(|resource| resource.raw())
        .unwrap_or_default();
    let task = event.task.map(|task| task.raw()).unwrap_or_default();
    let fault = event.fault.map(|fault| fault.raw()).unwrap_or_default();
    let detail = event.fault_detail.unwrap_or_default();
    let policy = event
        .fault_policy
        .map(|policy| policy.raw())
        .unwrap_or_default();
    let action = format_fault_policy_action(event.fault_policy_action);
    let message = event
        .message
        .map(|message| message.raw())
        .unwrap_or_default();

    format!(
        "event[{}] {} agent={} resource={} task={} fault={} detail={} policy={} action={} message={}",
        event.sequence, label, agent, resource, task, fault, detail, policy, action, message
    )
}

fn format_fault_policy_action(action: Option<FaultPolicyAction>) -> &'static str {
    match action {
        Some(FaultPolicyAction::RouteToHandler) => "route_to_handler",
        Some(FaultPolicyAction::RecoverTask) => "recover_task",
        None => "none",
    }
}
