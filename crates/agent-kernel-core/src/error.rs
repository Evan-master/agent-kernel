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
    AgentMismatch,
    ResourceMismatch,
    OperationDenied,
}
