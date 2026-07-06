//! Supervisor event formatting.
//!
//! This module belongs to the host-side `agent-supervisor` crate. It translates
//! kernel events into deterministic text output for simulator tests while
//! keeping formatting separate from the syscall flow in `main.rs`.

use agent_kernel_core::{Event, EventKind};

pub fn format_event(event: &Event) -> String {
    let agent = event.agent.raw();
    let resource = event
        .resource
        .map(|resource| resource.raw())
        .unwrap_or_default();

    match event.kind {
        EventKind::AgentRegistered => format_agent_event(event, "agent_registered"),
        EventKind::AgentSuspended => format_agent_event(event, "agent_suspended"),
        EventKind::AgentResumed => format_agent_event(event, "agent_resumed"),
        EventKind::AgentRetired => format_agent_event(event, "agent_retired"),
        EventKind::CapabilityGranted => format_capability_event(event, "capability_granted"),
        EventKind::CapabilityDerived => format_capability_event(event, "capability_derived"),
        EventKind::CapabilityRevoked => format_capability_event(event, "capability_revoked"),
        EventKind::IntentDeclared => format_intent_event(event, "intent_declared"),
        EventKind::IntentBound => format_intent_event(event, "intent_bound"),
        EventKind::IntentFulfilled => format_intent_event(event, "intent_fulfilled"),
        EventKind::IntentCancelled => format_intent_event(event, "intent_cancelled"),
        EventKind::Observation => {
            format!(
                "event[{}] observation agent={} resource={}",
                event.sequence, agent, resource
            )
        }
        EventKind::CheckpointCreated => {
            let checkpoint = event
                .checkpoint
                .map(|checkpoint| checkpoint.raw())
                .unwrap_or_default();
            format!(
                "event[{}] checkpoint agent={} resource={} checkpoint={}",
                event.sequence, agent, resource, checkpoint
            )
        }
        EventKind::RollbackRequested => {
            let checkpoint = event
                .checkpoint
                .map(|checkpoint| checkpoint.raw())
                .unwrap_or_default();
            format!(
                "event[{}] rollback agent={} resource={} checkpoint={}",
                event.sequence, agent, resource, checkpoint
            )
        }
        EventKind::ActionExecuted => {
            let action = event.action.map(|action| action.raw()).unwrap_or_default();
            format!(
                "event[{}] action agent={} resource={} action={}",
                event.sequence, agent, resource, action
            )
        }
        EventKind::VerificationRequested => {
            let action = event.action.map(|action| action.raw()).unwrap_or_default();
            format!(
                "event[{}] verification agent={} resource={} action={}",
                event.sequence, agent, resource, action
            )
        }
        EventKind::DelegationRequested => {
            let task = event.task.map(|task| task.raw()).unwrap_or_default();
            let target_agent = event
                .target_agent
                .map(|agent| agent.raw())
                .unwrap_or_default();
            format!(
                "event[{}] delegation agent={} resource={} task={} target_agent={}",
                event.sequence, agent, resource, task, target_agent
            )
        }
        EventKind::TaskCreated => format_task_event(event, "task_created"),
        EventKind::TaskAccepted => format_task_event(event, "task_accepted"),
        EventKind::TaskCompleted => format_task_event(event, "task_completed"),
        EventKind::TaskVerified => format_task_event(event, "task_verified"),
        EventKind::TaskCancelled => format_task_event(event, "task_cancelled"),
        EventKind::TaskQueued => format_task_event(event, "task_queued"),
        EventKind::TaskDispatched => format_task_event(event, "task_dispatched"),
        EventKind::TaskYielded => format_task_event(event, "task_yielded"),
        EventKind::MessageSent => format_message_event(event, "message_sent"),
        EventKind::MessageReceived => format_message_event(event, "message_received"),
        EventKind::MessageAcknowledged => format_message_event(event, "message_acknowledged"),
        EventKind::MemoryCellCreated => format_memory_event(event, "memory_cell_created"),
        EventKind::MemoryCellRecalled => format_memory_event(event, "memory_cell_recalled"),
        EventKind::MemoryCellRemembered => format_memory_event(event, "memory_cell_remembered"),
    }
}

fn format_agent_event(event: &Event, label: &str) -> String {
    let agent = event.agent.raw();
    let target_agent = event
        .target_agent
        .map(|agent| agent.raw())
        .unwrap_or_default();

    format!(
        "event[{}] {} agent={} target_agent={}",
        event.sequence, label, agent, target_agent
    )
}

fn format_intent_event(event: &Event, label: &str) -> String {
    let agent = event.agent.raw();
    let resource = event
        .resource
        .map(|resource| resource.raw())
        .unwrap_or_default();
    let intent = event.intent.map(|intent| intent.raw()).unwrap_or_default();

    format!(
        "event[{}] {} agent={} resource={} intent={}",
        event.sequence, label, agent, resource, intent
    )
}

fn format_task_event(event: &Event, label: &str) -> String {
    let agent = event.agent.raw();
    let resource = event
        .resource
        .map(|resource| resource.raw())
        .unwrap_or_default();
    let task = event.task.map(|task| task.raw()).unwrap_or_default();

    format!(
        "event[{}] {} agent={} resource={} task={}",
        event.sequence, label, agent, resource, task
    )
}

fn format_message_event(event: &Event, label: &str) -> String {
    let agent = event.agent.raw();
    let target_agent = event
        .target_agent
        .map(|agent| agent.raw())
        .unwrap_or_default();
    let message = event
        .message
        .map(|message| message.raw())
        .unwrap_or_default();

    format!(
        "event[{}] {} agent={} target_agent={} message={}",
        event.sequence, label, agent, target_agent, message
    )
}

fn format_memory_event(event: &Event, label: &str) -> String {
    let agent = event.agent.raw();
    let resource = event
        .resource
        .map(|resource| resource.raw())
        .unwrap_or_default();
    let memory_cell = event
        .memory_cell
        .map(|memory_cell| memory_cell.raw())
        .unwrap_or_default();

    format!(
        "event[{}] {} agent={} resource={} memory_cell={}",
        event.sequence, label, agent, resource, memory_cell
    )
}

fn format_capability_event(event: &Event, label: &str) -> String {
    let agent = event.agent.raw();
    let resource = event
        .resource
        .map(|resource| resource.raw())
        .unwrap_or_default();
    let capability = event
        .capability
        .map(|capability| capability.raw())
        .unwrap_or_default();

    format!(
        "event[{}] {} agent={} resource={} capability={}",
        event.sequence, label, agent, resource, capability
    )
}
