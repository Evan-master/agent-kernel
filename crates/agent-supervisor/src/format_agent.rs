//! Agent event formatting for deterministic simulator output.
//!
//! This supervisor-layer module owns display-only formatting for agent lifecycle
//! and launch events. It must not interpret authorization state or mutate kernel
//! records; it only renders the event fields recorded by the core.

use agent_kernel_core::Event;

pub fn format_agent_event(event: &Event, label: &str) -> String {
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

pub fn format_agent_launch_event(event: &Event) -> String {
    let agent = event.agent.raw();
    let resource = event
        .resource
        .map(|resource| resource.raw())
        .unwrap_or_default();
    let capability = event
        .capability
        .map(|capability| capability.raw())
        .unwrap_or_default();

    if let Some(task) = event.task {
        format!(
            "event[{}] agent_launched agent={} resource={} capability={} task={}",
            event.sequence,
            agent,
            resource,
            capability,
            task.raw()
        )
    } else {
        format!(
            "event[{}] agent_launched agent={} resource={} capability={}",
            event.sequence, agent, resource, capability
        )
    }
}
