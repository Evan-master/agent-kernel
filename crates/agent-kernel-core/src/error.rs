//! Explicit Agent Kernel error values.
//!
//! This module owns deterministic errors for capacity, lookup, authorization,
//! and revocation failures. Normal kernel failures must return these errors
//! instead of panicking.

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum KernelError {
    ResourceStoreFull,
    CapabilityStoreFull,
    EventLogFull,
    ResourceNotFound,
    CapabilityNotFound,
    CapabilityRevoked,
    CapabilityScopeMismatch,
    AgentMismatch,
    ResourceMismatch,
    OperationDenied,
    IntentStoreFull,
    IntentNotFound,
    IntentAgentMismatch,
    IntentStatusMismatch,
    TaskStoreFull,
    TaskNotFound,
    TaskAgentMismatch,
    TaskStatusMismatch,
    RunQueueFull,
    RunQueueEmpty,
    TaskNotRunnable,
    TaskAlreadyQueued,
}
