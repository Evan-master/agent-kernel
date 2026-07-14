//! Host-side formatting for deterministic simulator event output.

use agent_kernel_core::{Event, EventKind};

use crate::format_agent::{
    format_agent_event, format_agent_image_event, format_agent_launch_event,
};
use crate::format_driver::{
    format_device_event, format_driver_command_event, format_driver_event,
    format_driver_invocation_event,
};
use crate::format_fault::{
    format_fault_handler_event, format_fault_policy_apply_event, format_fault_policy_install_event,
    format_fault_route_event, format_task_fault_event,
};
use crate::format_signal::{format_signal_event, format_task_signal_event};

pub fn format_event(event: &Event) -> String {
    let agent = event.agent.raw();
    let resource = event
        .resource
        .map(|resource| resource.raw())
        .unwrap_or_default();

    match event.kind {
        EventKind::AgentRegistered => format_agent_event(event, "agent_registered"),
        EventKind::AgentImageRegistered => {
            format_agent_image_event(event, "agent_image_registered")
        }
        EventKind::AgentImageVerified => format_agent_image_event(event, "agent_image_verified"),
        EventKind::AgentImageRetired => format_agent_image_event(event, "agent_image_retired"),
        EventKind::AgentLaunched => format_agent_launch_event(event),
        EventKind::AgentSuspended => format_agent_event(event, "agent_suspended"),
        EventKind::AgentResumed => format_agent_event(event, "agent_resumed"),
        EventKind::AgentRetired => format_agent_event(event, "agent_retired"),
        EventKind::DriverEndpointRegistered => {
            format_capability_event(event, "driver_endpoint_registered")
        }
        EventKind::DriverBound => format_driver_event(event, "driver_bound"),
        EventKind::DeviceEventRaised => format_device_event(event, "device_event_raised"),
        EventKind::DeviceEventDelivered => format_device_event(event, "device_event_delivered"),
        EventKind::DeviceEventAcknowledged => {
            format_device_event(event, "device_event_acknowledged")
        }
        EventKind::DriverInvocationQueued => {
            format_driver_invocation_event(event, "driver_invocation_queued")
        }
        EventKind::DriverInvocationDispatched => {
            format_driver_invocation_event(event, "driver_invocation_dispatched")
        }
        EventKind::DriverInvocationTicked => {
            format_driver_invocation_event(event, "driver_invocation_ticked")
        }
        EventKind::DriverInvocationQuantumExpired => {
            format_driver_invocation_event(event, "driver_invocation_quantum_expired")
        }
        EventKind::DriverInvocationCompleted => {
            format_driver_invocation_event(event, "driver_invocation_completed")
        }
        EventKind::DriverCommandSubmitted => {
            format_driver_command_event(event, "driver_command_submitted")
        }
        EventKind::DriverCommandDispatched => {
            format_driver_command_event(event, "driver_command_dispatched")
        }
        EventKind::DriverCommandCompleted => {
            format_driver_command_event(event, "driver_command_completed")
        }
        EventKind::DriverCommandFailed => {
            format_driver_command_event(event, "driver_command_failed")
        }
        EventKind::ResourceCreated => format_capability_event(event, "resource_created"),
        EventKind::ResourceRetired => format_capability_event(event, "resource_retired"),
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
        EventKind::TaskTicked => format_task_tick_event(event, "task_ticked"),
        EventKind::TaskQuantumExpired => format_task_tick_event(event, "task_quantum_expired"),
        EventKind::TaskWaiting => format_task_signal_event(event, "task_waiting"),
        EventKind::TaskWoken => format_signal_event(event, "task_woken"),
        EventKind::TaskFaulted => format_task_fault_event(event, "task_faulted"),
        EventKind::TaskFaultRecovered => format_task_fault_event(event, "task_fault_recovered"),
        EventKind::SignalEmitted => format_signal_event(event, "signal_emitted"),
        EventKind::FaultHandlerInstalled => {
            format_fault_handler_event(event, "fault_handler_installed")
        }
        EventKind::FaultRouted => format_fault_route_event(event, "fault_routed"),
        EventKind::FaultPolicyInstalled => {
            format_fault_policy_install_event(event, "fault_policy_installed")
        }
        EventKind::FaultPolicyApplied => {
            format_fault_policy_apply_event(event, "fault_policy_applied")
        }
        EventKind::MessageSent => format_message_event(event, "message_sent"),
        EventKind::MessageReceived => format_message_event(event, "message_received"),
        EventKind::MessageAcknowledged => format_message_event(event, "message_acknowledged"),
        EventKind::MemoryCellCreated => format_memory_event(event, "memory_cell_created"),
        EventKind::MemoryCellRecalled => format_memory_event(event, "memory_cell_recalled"),
        EventKind::MemoryCellRemembered => format_memory_event(event, "memory_cell_remembered"),
        EventKind::NamespaceEntryBound => format_namespace_event(event, "namespace_entry_bound"),
        EventKind::NamespaceEntryResolved => {
            format_namespace_event(event, "namespace_entry_resolved")
        }
        EventKind::NamespaceEntryRebound => {
            format_namespace_event(event, "namespace_entry_rebound")
        }
    }
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

fn format_task_tick_event(event: &Event, label: &str) -> String {
    let agent = event.agent.raw();
    let resource = event
        .resource
        .map(|resource| resource.raw())
        .unwrap_or_default();
    let task = event.task.map(|task| task.raw()).unwrap_or_default();
    let ticks = event.task_ticks.unwrap_or_default();
    let quantum = event.task_quantum.unwrap_or_default();

    format!(
        "event[{}] {} agent={} resource={} task={} ticks={} quantum={}",
        event.sequence, label, agent, resource, task, ticks, quantum
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

fn format_namespace_event(event: &Event, label: &str) -> String {
    let agent = event.agent.raw();
    let resource = event
        .resource
        .map(|resource| resource.raw())
        .unwrap_or_default();
    let namespace_entry = event
        .namespace_entry
        .map(|entry| entry.raw())
        .unwrap_or_default();
    let key = event.namespace_key.map(|key| key.raw()).unwrap_or_default();

    format!(
        "event[{}] {} agent={} resource={} namespace_entry={} key={}",
        event.sequence, label, agent, resource, namespace_entry, key
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
