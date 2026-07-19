//! Deterministic serial formatting for Agent Kernel boot events.
//!
//! This architecture-binary module owns only the QEMU-readable event labels.
//! It receives immutable kernel events and performs no semantic mutation.

use agent_kernel_core::{Event, EventKind};

use crate::{serial_write_line, serial_write_str, serial_write_u64};

pub(super) fn write(events: &[Event]) {
    for event in events {
        serial_write_str("event[");
        serial_write_u64(event.sequence);
        serial_write_str("] ");
        serial_write_line(label(event.kind));
    }
}

const fn label(kind: EventKind) -> &'static str {
    match kind {
        EventKind::AgentRegistered => "agent_registered",
        EventKind::AgentImageRegistered => "agent_image_registered",
        EventKind::AgentImageVerified => "agent_image_verified",
        EventKind::AgentImageRetired => "agent_image_retired",
        EventKind::AgentLaunched => "agent_launched",
        EventKind::AgentEntryRetired => "agent_entry_retired",
        EventKind::RuntimeAdmissionRequested => "runtime_admission_requested",
        EventKind::RuntimeAdmissionAdmitted => "runtime_admission_admitted",
        EventKind::RuntimeAdmissionRejected => "runtime_admission_rejected",
        EventKind::RuntimeAdmissionReleased => "runtime_admission_released",
        EventKind::RuntimeAdmissionCompacted => "runtime_admission_compacted",
        EventKind::AgentSuspended => "agent_suspended",
        EventKind::AgentResumed => "agent_resumed",
        EventKind::AgentRetired => "agent_retired",
        EventKind::AgentRecordRetired => "agent_record_retired",
        EventKind::DriverEndpointRegistered => "driver_endpoint_registered",
        EventKind::DriverBound => "driver_bound",
        EventKind::DeviceEventRaised => "device_event_raised",
        EventKind::DeviceEventDelivered => "device_event_delivered",
        EventKind::DeviceEventAcknowledged => "device_event_acknowledged",
        EventKind::DriverInvocationQueued => "driver_invocation_queued",
        EventKind::DriverInvocationDispatched => "driver_invocation_dispatched",
        EventKind::DriverInvocationTicked => "driver_invocation_ticked",
        EventKind::DriverInvocationQuantumExpired => "driver_invocation_quantum_expired",
        EventKind::DriverInvocationCompleted => "driver_invocation_completed",
        EventKind::DriverCommandSubmitted => "driver_command_submitted",
        EventKind::DriverCommandDispatched => "driver_command_dispatched",
        EventKind::DriverCommandCompleted => "driver_command_completed",
        EventKind::DriverCommandFailed => "driver_command_failed",
        EventKind::ResourceCreated => "resource_created",
        EventKind::ResourceRetired => "resource_retired",
        EventKind::CapabilityGranted => "capability_granted",
        EventKind::CapabilityDerived => "capability_derived",
        EventKind::CapabilityRevoked => "capability_revoked",
        EventKind::CapabilityCompacted => "capability_compacted",
        EventKind::IntentDeclared => "intent_declared",
        EventKind::IntentBound => "intent_bound",
        EventKind::IntentFulfilled => "intent_fulfilled",
        EventKind::IntentCancelled => "intent_cancelled",
        EventKind::IntentCompacted => "intent_compacted",
        EventKind::Observation => "observation",
        EventKind::ActionExecuted => "action",
        EventKind::VerificationRequested => "verification",
        EventKind::CheckpointCreated => "checkpoint",
        EventKind::RollbackRequested => "rollback",
        EventKind::DelegationRequested => "delegation",
        EventKind::TaskCreated => "task_created",
        EventKind::TaskAccepted => "task_accepted",
        EventKind::TaskResultSubmitted => "task_result_submitted",
        EventKind::TaskResultInspected => "task_result_inspected",
        EventKind::TaskCompleted => "task_completed",
        EventKind::TaskVerified => "task_verified",
        EventKind::TaskCancelled => "task_cancelled",
        EventKind::TaskCompacted => "task_compacted",
        EventKind::TaskQueued => "task_queued",
        EventKind::TaskDispatched => "task_dispatched",
        EventKind::TaskYielded => "task_yielded",
        EventKind::TaskTicked => "task_ticked",
        EventKind::TaskQuantumExpired => "task_quantum_expired",
        EventKind::TaskWaiting => "task_waiting",
        EventKind::TaskWoken => "task_woken",
        EventKind::TaskFaulted => "task_faulted",
        EventKind::TaskFaultRecovered => "task_fault_recovered",
        EventKind::SignalEmitted => "signal_emitted",
        EventKind::FaultHandlerInstalled => "fault_handler_installed",
        EventKind::FaultRouted => "fault_routed",
        EventKind::FaultPolicyInstalled => "fault_policy_installed",
        EventKind::FaultPolicyApplied => "fault_policy_applied",
        EventKind::MessageSent => "message_sent",
        EventKind::MessageWaitStarted => "message_wait_started",
        EventKind::MessageWaitWoken => "message_wait_woken",
        EventKind::MessageReceived => "message_received",
        EventKind::MessageAcknowledged => "message_acknowledged",
        EventKind::MessageRetired => "message_retired",
        EventKind::OrphanedMessageRetired => "orphaned_message_retired",
        EventKind::MemoryCellCreated => "memory_cell_created",
        EventKind::MemoryCellRecalled => "memory_cell_recalled",
        EventKind::MemoryCellRemembered => "memory_cell_remembered",
        EventKind::NamespaceEntryBound => "namespace_entry_bound",
        EventKind::NamespaceEntryResolved => "namespace_entry_resolved",
        EventKind::NamespaceEntryRebound => "namespace_entry_rebound",
    }
}
