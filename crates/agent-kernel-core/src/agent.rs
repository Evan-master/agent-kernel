//! Kernel-owned agent records.
//!
//! This module belongs to `agent-kernel-core`. It defines copyable agent
//! records for the fixed-capacity no_std agent registry. It does not contain
//! prompts, model sessions, host process data, or scheduling policy.

use crate::{AgentExecutionContext, AgentId, CapabilityId, ResourceId};

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum AgentStatus {
    Active,
    Suspended,
    Retired,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct AgentRecord {
    pub id: AgentId,
    pub status: AgentStatus,
    pub manager: Option<AgentId>,
    pub management_resource: Option<ResourceId>,
}

impl AgentRecord {
    pub(crate) const fn empty() -> Self {
        Self {
            id: AgentId::new(0),
            status: AgentStatus::Active,
            manager: None,
            management_resource: None,
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct AgentRecordRetirement {
    record: AgentRecord,
    context: AgentExecutionContext,
    actor: AgentId,
    authority: CapabilityId,
    management_resource: ResourceId,
    retired_floor: AgentId,
}

impl AgentRecordRetirement {
    pub(crate) const fn new(
        record: AgentRecord,
        context: AgentExecutionContext,
        actor: AgentId,
        authority: CapabilityId,
        management_resource: ResourceId,
        retired_floor: AgentId,
    ) -> Self {
        Self {
            record,
            context,
            actor,
            authority,
            management_resource,
            retired_floor,
        }
    }

    pub const fn record(self) -> AgentRecord {
        self.record
    }

    pub const fn context(self) -> AgentExecutionContext {
        self.context
    }

    pub const fn agent(self) -> AgentId {
        self.record.id
    }

    pub const fn actor(self) -> AgentId {
        self.actor
    }

    pub const fn authority(self) -> CapabilityId {
        self.authority
    }

    pub const fn management_resource(self) -> ResourceId {
        self.management_resource
    }

    pub const fn retired_floor(self) -> AgentId {
        self.retired_floor
    }
}
