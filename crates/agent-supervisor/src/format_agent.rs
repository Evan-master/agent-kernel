//! Agent event formatting for deterministic simulator output.
//!
//! This supervisor-layer module owns display-only formatting for agent lifecycle
//! and launch events. It must not interpret authorization state or mutate kernel
//! records; it only renders the event fields recorded by the core.

use agent_kernel_core::{AgentImageKind, Event};
use std::fmt::Write;

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

pub fn format_agent_entry_retirement_event(event: &Event) -> String {
    let target = event
        .target_agent
        .map(|agent| agent.raw())
        .unwrap_or_default();
    let resource = event
        .resource
        .map(|resource| resource.raw())
        .unwrap_or_default();
    let capability = event
        .capability
        .map(|capability| capability.raw())
        .unwrap_or_default();
    let authority = event
        .source_capability
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
    let intent = event.intent.map(|intent| intent.raw()).unwrap_or_default();
    let task = event.task.map(|task| task.raw()).unwrap_or_default();

    format!(
        "event[{}] agent_entry_retired actor={} target_agent={} resource={} capability={} authority={} image={} kind={} intent={} task={}",
        event.sequence,
        event.agent.raw(),
        target,
        resource,
        capability,
        authority,
        image,
        kind,
        intent,
        task
    )
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

pub fn format_agent_image_retirement_event(event: &Event) -> String {
    let owner = event
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
    let image = event
        .agent_image
        .map(|image| image.raw())
        .unwrap_or_default();
    let kind = event
        .agent_image_kind
        .map(image_kind_label)
        .unwrap_or("unknown");
    let digest = event
        .agent_image_digest
        .map(|digest| format_digest(digest.bytes))
        .unwrap_or_else(|| String::from("unknown"));
    let abi = event.agent_image_abi_version.unwrap_or_default();
    let entry = event.agent_image_entry_version.unwrap_or_default();

    format!(
        "event[{}] agent_image_record_retired actor={} owner={} resource={} authority={} image={} kind={} digest={} abi={} entry={}",
        event.sequence,
        event.agent.raw(),
        owner,
        resource,
        authority,
        image,
        kind,
        digest,
        abi,
        entry
    )
}

fn format_digest(bytes: [u8; 32]) -> String {
    let mut output = String::with_capacity(64);
    for byte in bytes {
        write!(&mut output, "{byte:02x}").expect("writing to a String cannot fail");
    }
    output
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

#[cfg(test)]
mod tests {
    use agent_kernel_core::{
        AgentId, AgentImageDigest, AgentImageKind, KernelCore, Operation, OperationSet,
        ResourceKind,
    };

    use super::format_agent_image_retirement_event;

    type TestCore =
        KernelCore<4, 3, 12, 32, 0, 32, 0, 4, 4, 4, 0, 0, 0, 0, 0, 0, 0, 4, 0, 0, 0, 0, 4>;

    #[test]
    fn image_record_retirement_format_preserves_complete_audit_identity() {
        let mut core = TestCore::new();
        let actor = AgentId::new(1);
        core.register_agent(actor).unwrap();
        let resource = core
            .register_resource(ResourceKind::Workspace, None)
            .unwrap();
        let operations = OperationSet::only(Operation::Observe)
            .with(Operation::Act)
            .with(Operation::Verify)
            .with(Operation::Rollback)
            .with(Operation::Delegate);
        let authority = core.grant_capability(actor, resource, operations).unwrap();
        let image = core
            .register_agent_image(
                actor,
                authority,
                resource,
                AgentImageKind::Worker,
                AgentImageDigest::new([0xab; 32]),
                7,
                9,
            )
            .unwrap();
        core.retire_agent_image(actor, authority, image).unwrap();
        core.retire_agent_image_record(actor, authority, image)
            .unwrap();

        assert_eq!(
            format_agent_image_retirement_event(core.events().last().unwrap()),
            "event[5] agent_image_record_retired actor=1 owner=1 resource=1 authority=1 image=1 kind=worker digest=abababababababababababababababababababababababababababababababab abi=7 entry=9"
        );
    }
}
