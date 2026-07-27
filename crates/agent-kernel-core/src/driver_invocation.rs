//! Kernel-owned Driver Agent invocation records.
//!
//! This module belongs to `agent-kernel-core`. It defines fixed-width work
//! records created from delivered device events. Queueing and scheduling live
//! in separate runtime modules; this file performs no I/O or allocation.

use crate::{AgentId, DeviceEventId, DriverBindingId, DriverInvocationId, FaultKind, ResourceId};

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum DriverInvocationStatus {
    Queued,
    Running,
    Faulted,
    Completed,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct DriverInvocationRecord {
    pub id: DriverInvocationId,
    pub binding: DriverBindingId,
    pub driver: AgentId,
    pub resource: ResourceId,
    pub event: DeviceEventId,
    pub status: DriverInvocationStatus,
    pub run_ticks: u64,
    pub quantum_remaining: u64,
    pub fault_kind: Option<FaultKind>,
    pub fault_detail: Option<u64>,
    pub restart_generation: u8,
}

impl DriverInvocationRecord {
    pub(crate) const fn empty() -> Self {
        Self {
            id: DriverInvocationId::new(0),
            binding: DriverBindingId::new(0),
            driver: AgentId::new(0),
            resource: ResourceId::new(0),
            event: DeviceEventId::new(0),
            status: DriverInvocationStatus::Completed,
            run_ticks: 0,
            quantum_remaining: 0,
            fault_kind: None,
            fault_detail: None,
            restart_generation: 0,
        }
    }
}
