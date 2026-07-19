//! Supervisor formatting for fault-specific events.
//!
//! This module belongs to the host-side `agent-supervisor` crate. It keeps
//! task fault, handler install, and route formatting out of the general event
//! formatter while preserving deterministic text output for tests.

use agent_kernel_core::{Event, FaultKind};

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

pub(crate) fn format_fault_compaction_event(event: &Event) -> String {
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
    let fault = event.fault.map(|fault| fault.raw()).unwrap_or_default();
    let kind = event.fault_kind.map(fault_kind_label).unwrap_or("unknown");
    let detail = event.fault_detail.unwrap_or_default();

    format!(
        "event[{}] fault_compacted actor={} target_agent={} resource={} authority={} task={} fault={} kind={} detail={}",
        event.sequence,
        event.agent.raw(),
        target_agent,
        resource,
        authority,
        task,
        fault,
        kind,
        detail
    )
}

fn fault_kind_label(kind: FaultKind) -> &'static str {
    match kind {
        FaultKind::ExecutionTrap => "execution_trap",
        FaultKind::AuthorityViolation => "authority_violation",
        FaultKind::ResourceFault => "resource_fault",
        FaultKind::VerificationFault => "verification_fault",
    }
}

fn format_fault_policy_action(action: Option<FaultPolicyAction>) -> &'static str {
    match action {
        Some(FaultPolicyAction::RouteToHandler) => "route_to_handler",
        Some(FaultPolicyAction::RecoverTask) => "recover_task",
        None => "none",
    }
}

#[cfg(test)]
mod tests {
    use agent_kernel_core::{
        AgentId, CapabilityId, EventKind, FaultId, KernelCore, ResourceId, TaskId,
    };

    use super::*;

    #[test]
    fn fault_compaction_format_preserves_complete_audit_identity() {
        let mut core = KernelCore::<1, 0, 0, 1, 0, 0, 0, 0, 0, 0>::new();
        core.register_agent(AgentId::new(12)).unwrap();
        let mut event = core.events()[0];
        event.sequence = 42;
        event.agent = AgentId::new(12);
        event.kind = EventKind::FaultCompacted;
        event.target_agent = Some(AgentId::new(6));
        event.resource = Some(ResourceId::new(1));
        event.capability = Some(CapabilityId::new(23));
        event.task = Some(TaskId::new(3));
        event.fault = Some(FaultId::new(4));
        event.fault_kind = Some(FaultKind::VerificationFault);
        event.fault_detail = Some(99);

        assert_eq!(
            format_fault_compaction_event(&event),
            "event[42] fault_compacted actor=12 target_agent=6 resource=1 authority=23 task=3 fault=4 kind=verification_fault detail=99"
        );
    }
}
