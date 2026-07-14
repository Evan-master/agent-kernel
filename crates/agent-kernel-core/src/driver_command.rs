//! Fixed-width native driver command records.
//!
//! This module belongs to `agent-kernel-core`. It defines allocator-free
//! command values shared by the command state machine, syscall facade, event
//! log, and supervisor. It does not dispatch commands or perform device I/O.

use crate::{
    AgentId, DeviceEventId, DriverBindingId, DriverCommandId, DriverInvocationId, ResourceId,
};

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum DriverCommandKind {
    Configure,
    Read,
    Write,
    Reset,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct DriverCommandPayload {
    pub opcode: u16,
    pub value: u64,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct DriverCommandResult {
    pub code: u16,
    pub value: u64,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum DriverCommandStatus {
    Submitted,
    Dispatched,
    Completed,
    Failed,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct DriverCommandRequest {
    pub command: DriverCommandId,
    pub binding: DriverBindingId,
    pub resource: ResourceId,
    pub driver: AgentId,
    pub cause: Option<DeviceEventId>,
    pub invocation: Option<DriverInvocationId>,
    pub kind: DriverCommandKind,
    pub payload: DriverCommandPayload,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct DriverCommandRecord {
    pub id: DriverCommandId,
    pub binding: DriverBindingId,
    pub resource: ResourceId,
    pub driver: AgentId,
    pub cause: Option<DeviceEventId>,
    pub invocation: Option<DriverInvocationId>,
    pub kind: DriverCommandKind,
    pub payload: DriverCommandPayload,
    pub status: DriverCommandStatus,
    pub result: Option<DriverCommandResult>,
}

impl DriverCommandRecord {
    pub(crate) const fn empty() -> Self {
        Self {
            id: DriverCommandId::new(0),
            binding: DriverBindingId::new(0),
            resource: ResourceId::new(0),
            driver: AgentId::new(0),
            cause: None,
            invocation: None,
            kind: DriverCommandKind::Configure,
            payload: DriverCommandPayload {
                opcode: 0,
                value: 0,
            },
            status: DriverCommandStatus::Failed,
            result: None,
        }
    }
}
