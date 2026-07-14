//! Host-side formatting for driver bindings, device events, and commands.
//!
//! This supervisor module renders native driver lifecycle records for the
//! deterministic simulator trace. It owns no kernel state and performs no
//! device I/O.

use agent_kernel_core::{
    DeviceEventKind, DeviceEventPayload, DriverCommandKind, DriverCommandPayload, Event,
};

pub fn format_driver_event(event: &Event, label: &str) -> String {
    let agent = event.agent.raw();
    let resource = event
        .resource
        .map(|resource| resource.raw())
        .unwrap_or_default();
    let capability = event
        .capability
        .map(|capability| capability.raw())
        .unwrap_or_default();
    let binding = event
        .driver_binding
        .map(|binding| binding.raw())
        .unwrap_or_default();
    let target_agent = event
        .target_agent
        .map(|agent| agent.raw())
        .unwrap_or_default();

    format!(
        "event[{}] {} agent={} resource={} capability={} driver_binding={} target_agent={}",
        event.sequence, label, agent, resource, capability, binding, target_agent
    )
}

pub fn format_device_event(event: &Event, label: &str) -> String {
    let agent = event.agent.raw();
    let resource = event
        .resource
        .map(|resource| resource.raw())
        .unwrap_or_default();
    let capability = event
        .capability
        .map(|capability| capability.raw())
        .unwrap_or_default();
    let binding = event
        .driver_binding
        .map(|binding| binding.raw())
        .unwrap_or_default();
    let device_event = event
        .device_event
        .map(|device_event| device_event.raw())
        .unwrap_or_default();
    let kind = event
        .device_event_kind
        .map(format_device_event_kind)
        .unwrap_or("unknown");
    let payload = event
        .device_event_payload
        .unwrap_or(DeviceEventPayload { code: 0, value: 0 });

    format!(
        "event[{}] {} agent={} resource={} capability={} driver_binding={} device_event={} kind={} code={} value={}",
        event.sequence,
        label,
        agent,
        resource,
        capability,
        binding,
        device_event,
        kind,
        payload.code,
        payload.value
    )
}

pub fn format_driver_command_event(event: &Event, label: &str) -> String {
    let agent = event.agent.raw();
    let resource = event
        .resource
        .map(|resource| resource.raw())
        .unwrap_or_default();
    let capability = event
        .capability
        .map(|capability| capability.raw())
        .unwrap_or_default();
    let binding = event
        .driver_binding
        .map(|binding| binding.raw())
        .unwrap_or_default();
    let cause = event
        .device_event
        .map(|device_event| device_event.raw())
        .unwrap_or_default();
    let command = event
        .driver_command
        .map(|command| command.raw())
        .unwrap_or_default();
    let kind = event
        .driver_command_kind
        .map(format_driver_command_kind)
        .unwrap_or("unknown");
    let payload = event
        .driver_command_payload
        .unwrap_or(DriverCommandPayload {
            opcode: 0,
            value: 0,
        });

    if let Some(result) = event.driver_command_result {
        format!(
            "event[{}] {} agent={} resource={} capability={} driver_binding={} device_event={} driver_command={} kind={} opcode={} value={} result_code={} result_value={}",
            event.sequence,
            label,
            agent,
            resource,
            capability,
            binding,
            cause,
            command,
            kind,
            payload.opcode,
            payload.value,
            result.code,
            result.value
        )
    } else {
        format!(
            "event[{}] {} agent={} resource={} capability={} driver_binding={} device_event={} driver_command={} kind={} opcode={} value={}",
            event.sequence,
            label,
            agent,
            resource,
            capability,
            binding,
            cause,
            command,
            kind,
            payload.opcode,
            payload.value
        )
    }
}

fn format_device_event_kind(kind: DeviceEventKind) -> &'static str {
    match kind {
        DeviceEventKind::Interrupt => "interrupt",
        DeviceEventKind::DataReady => "data_ready",
        DeviceEventKind::Fault => "fault",
        DeviceEventKind::StateChanged => "state_changed",
    }
}

fn format_driver_command_kind(kind: DriverCommandKind) -> &'static str {
    match kind {
        DriverCommandKind::Configure => "configure",
        DriverCommandKind::Read => "read",
        DriverCommandKind::Write => "write",
        DriverCommandKind::Reset => "reset",
    }
}
