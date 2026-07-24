//! Stable numeric tags for the Event archive format.

use crate::{
    AgentImageKind, AgentImageSignerStatus, DeviceEventKind, DriverCommandKind, EventKind,
    FaultKind, FaultPolicyAction, IntentKind, MessageKind, Operation, VerificationRequirement,
    WaiterKind,
};

pub(super) const fn event_kind(value: EventKind) -> u16 {
    match value {
        EventKind::AgentRegistered => 1,
        EventKind::AgentImageRegistered => 2,
        EventKind::AgentImageVerified => 3,
        EventKind::AgentImageRetired => 4,
        EventKind::AgentImageRecordRetired => 5,
        EventKind::AgentImageSignerTrusted => 88,
        EventKind::AgentImageSignerRevoked => 89,
        EventKind::AgentLaunched => 6,
        EventKind::AgentEntryRetired => 7,
        EventKind::RuntimeAdmissionRequested => 8,
        EventKind::RuntimeAdmissionAdmitted => 9,
        EventKind::RuntimeAdmissionRejected => 10,
        EventKind::RuntimeAdmissionReleased => 11,
        EventKind::RuntimeAdmissionCompacted => 12,
        EventKind::AgentSuspended => 13,
        EventKind::AgentResumed => 14,
        EventKind::AgentRetired => 15,
        EventKind::AgentRecordRetired => 16,
        EventKind::DriverEndpointRegistered => 17,
        EventKind::DriverBound => 18,
        EventKind::DeviceEventRaised => 19,
        EventKind::DeviceEventDelivered => 20,
        EventKind::DeviceEventAcknowledged => 21,
        EventKind::DriverCommandSubmitted => 22,
        EventKind::DriverCommandDispatched => 23,
        EventKind::DriverCommandCompleted => 24,
        EventKind::DriverCommandFailed => 25,
        EventKind::DriverInvocationQueued => 26,
        EventKind::DriverInvocationDispatched => 27,
        EventKind::DriverInvocationTicked => 28,
        EventKind::DriverInvocationQuantumExpired => 29,
        EventKind::DriverInvocationCompleted => 30,
        EventKind::ResourceCreated => 31,
        EventKind::ResourceRetired => 32,
        EventKind::ResourceRecordRetired => 85,
        EventKind::CapabilityGranted => 33,
        EventKind::CapabilityDerived => 34,
        EventKind::CapabilityRevoked => 35,
        EventKind::CapabilityCompacted => 36,
        EventKind::IntentDeclared => 37,
        EventKind::IntentBound => 38,
        EventKind::IntentFulfilled => 39,
        EventKind::IntentCancelled => 40,
        EventKind::IntentCompacted => 41,
        EventKind::Observation => 42,
        EventKind::ActionExecuted => 43,
        EventKind::VerificationRequested => 44,
        EventKind::CheckpointCreated => 45,
        EventKind::RollbackRequested => 46,
        EventKind::DelegationRequested => 47,
        EventKind::TaskCreated => 48,
        EventKind::TaskAccepted => 49,
        EventKind::TaskResultSubmitted => 50,
        EventKind::TaskResultInspected => 51,
        EventKind::TaskCompleted => 52,
        EventKind::TaskVerified => 53,
        EventKind::TaskCancelled => 54,
        EventKind::TaskCompacted => 55,
        EventKind::TaskQueued => 56,
        EventKind::TaskDispatched => 57,
        EventKind::TaskYielded => 58,
        EventKind::TaskTicked => 59,
        EventKind::TaskQuantumExpired => 60,
        EventKind::TaskWaiting => 61,
        EventKind::TaskWoken => 62,
        EventKind::TaskFaulted => 63,
        EventKind::TaskFaultRecovered => 64,
        EventKind::SignalEmitted => 65,
        EventKind::FaultHandlerInstalled => 66,
        EventKind::FaultRouted => 67,
        EventKind::FaultPolicyInstalled => 68,
        EventKind::FaultPolicyApplied => 69,
        EventKind::FaultCompacted => 70,
        EventKind::MessageSent => 71,
        EventKind::MessageWaitStarted => 72,
        EventKind::MessageWaitWoken => 73,
        EventKind::MessageReceived => 74,
        EventKind::MessageAcknowledged => 75,
        EventKind::MessageRetired => 76,
        EventKind::OrphanedMessageRetired => 77,
        EventKind::WaiterCompacted => 78,
        EventKind::MemoryCellCreated => 79,
        EventKind::MemoryCellRecalled => 80,
        EventKind::MemoryCellRemembered => 81,
        EventKind::MemoryCellRecordRetired => 86,
        EventKind::NamespaceEntryBound => 82,
        EventKind::NamespaceEntryResolved => 83,
        EventKind::NamespaceEntryRebound => 84,
        EventKind::NamespaceEntryRetired => 87,
    }
}

pub(super) const fn intent_kind(value: IntentKind) -> u16 {
    match value {
        IntentKind::Observe => 1,
        IntentKind::Act => 2,
        IntentKind::Verify => 3,
        IntentKind::Checkpoint => 4,
        IntentKind::Rollback => 5,
    }
}

pub(super) const fn message_kind(value: MessageKind) -> u16 {
    match value {
        MessageKind::Notify => 1,
        MessageKind::Request => 2,
        MessageKind::Response => 3,
        MessageKind::Fault => 4,
    }
}

pub(super) const fn operation(value: Operation) -> u16 {
    match value {
        Operation::Observe => 1,
        Operation::Act => 2,
        Operation::Verify => 3,
        Operation::Checkpoint => 4,
        Operation::Rollback => 5,
        Operation::Delegate => 6,
    }
}

pub(super) const fn verification(value: VerificationRequirement) -> u16 {
    match value {
        VerificationRequirement::Optional => 1,
        VerificationRequirement::Required => 2,
    }
}

pub(super) const fn fault_kind(value: FaultKind) -> u16 {
    match value {
        FaultKind::ExecutionTrap => 1,
        FaultKind::AuthorityViolation => 2,
        FaultKind::ResourceFault => 3,
        FaultKind::VerificationFault => 4,
    }
}

pub(super) const fn fault_policy_action(value: FaultPolicyAction) -> u16 {
    match value {
        FaultPolicyAction::RouteToHandler => 1,
        FaultPolicyAction::RecoverTask => 2,
    }
}

pub(super) const fn waiter_kind(value: WaiterKind) -> u16 {
    match value {
        WaiterKind::Signal => 1,
        WaiterKind::Mailbox => 2,
    }
}

pub(super) const fn device_event_kind(value: DeviceEventKind) -> u16 {
    match value {
        DeviceEventKind::Interrupt => 1,
        DeviceEventKind::DataReady => 2,
        DeviceEventKind::Fault => 3,
        DeviceEventKind::StateChanged => 4,
    }
}

pub(super) const fn driver_command_kind(value: DriverCommandKind) -> u16 {
    match value {
        DriverCommandKind::Configure => 1,
        DriverCommandKind::Read => 2,
        DriverCommandKind::Write => 3,
        DriverCommandKind::Reset => 4,
    }
}

pub(super) const fn agent_image_kind(value: AgentImageKind) -> u16 {
    match value {
        AgentImageKind::Bootstrap => 1,
        AgentImageKind::Supervisor => 2,
        AgentImageKind::Worker => 3,
        AgentImageKind::Verifier => 4,
        AgentImageKind::FaultHandler => 5,
        AgentImageKind::Driver => 6,
        AgentImageKind::StateSigner => 7,
    }
}

pub(super) const fn agent_image_signer_status(value: AgentImageSignerStatus) -> u16 {
    match value {
        AgentImageSignerStatus::Active => 1,
        AgentImageSignerStatus::Revoked => 2,
    }
}
