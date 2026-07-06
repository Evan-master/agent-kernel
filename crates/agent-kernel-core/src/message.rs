//! Kernel-owned message model for native agent IPC.
//!
//! This module belongs to `agent-kernel-core`. It defines copyable message
//! records and typed payload references for the fixed-capacity no_std mailbox
//! store. It deliberately carries kernel object IDs instead of heap-allocated
//! bytes or host transport handles.

use crate::{ActionId, AgentId, CapabilityId, FaultId, IntentId, MessageId, ResourceId, TaskId};

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum MessageKind {
    Notify,
    Request,
    Response,
    Fault,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum MessageStatus {
    Pending,
    Received,
    Acknowledged,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct MessagePayload {
    pub resource: Option<ResourceId>,
    pub capability: Option<CapabilityId>,
    pub intent: Option<IntentId>,
    pub task: Option<TaskId>,
    pub action: Option<ActionId>,
    pub fault: Option<FaultId>,
}

impl MessagePayload {
    pub const fn empty() -> Self {
        Self {
            resource: None,
            capability: None,
            intent: None,
            task: None,
            action: None,
            fault: None,
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct MessageRecord {
    pub id: MessageId,
    pub sender: AgentId,
    pub recipient: AgentId,
    pub kind: MessageKind,
    pub payload: MessagePayload,
    pub status: MessageStatus,
}

impl MessageRecord {
    pub(crate) const fn empty() -> Self {
        Self {
            id: MessageId::new(0),
            sender: AgentId::new(0),
            recipient: AgentId::new(0),
            kind: MessageKind::Notify,
            payload: MessagePayload::empty(),
            status: MessageStatus::Acknowledged,
        }
    }
}
