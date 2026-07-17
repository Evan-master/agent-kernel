//! Agent event formatting for deterministic simulator output.
//!
//! This supervisor-layer module owns display-only formatting for agent lifecycle
//! and launch events. It must not interpret authorization state or mutate kernel
//! records; it only renders the event fields recorded by the core.

use agent_kernel_core::{AgentImageKind, Event};

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
    let image = event
        .agent_image
        .map(|image| image.raw())
        .unwrap_or_default();

    if let Some(task) = event.task {
        format!(
            "event[{}] agent_launched agent={} resource={} capability={} image={} task={}",
            event.sequence,
            agent,
            resource,
            capability,
            image,
            task.raw()
        )
    } else {
        format!(
            "event[{}] agent_launched agent={} resource={} capability={} image={}",
            event.sequence, agent, resource, capability, image
        )
    }
}

pub fn format_agent_image_event(event: &Event, label: &str) -> String {
    let agent = event.agent.raw();
    let resource = event
        .resource
        .map(|resource| resource.raw())
        .unwrap_or_default();
    let capability = event
        .capability
        .map(|capability| capability.raw())
        .unwrap_or_default();
    let image = event
        .agent_image
        .map(|image| image.raw())
        .unwrap_or_default();
    let kind = event
        .agent_image_kind
        .map(image_kind_label)
        .unwrap_or("unknown");

    format!(
        "event[{}] {} agent={} resource={} capability={} image={} kind={}",
        event.sequence, label, agent, resource, capability, image, kind
    )
}

fn image_kind_label(kind: AgentImageKind) -> &'static str {
    match kind {
        AgentImageKind::Bootstrap => "bootstrap",
        AgentImageKind::Supervisor => "supervisor",
        AgentImageKind::Worker => "worker",
        AgentImageKind::Verifier => "verifier",
        AgentImageKind::FaultHandler => "fault_handler",
        AgentImageKind::Driver => "driver",
    }
}
