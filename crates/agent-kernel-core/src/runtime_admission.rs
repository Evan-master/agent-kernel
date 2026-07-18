//! Runtime admission records and generation-bound permits.
//!
//! These copyable values bridge an audited Supervisor request to a later
//! platform admission attempt without exposing mutable core storage.

use crate::{AgentId, AgentImageId, CapabilityId, ResourceId, RuntimeAdmissionId, TaskId};

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum RuntimeAdmissionStatus {
    Requested,
    Admitted,
    Rejected,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum RuntimeAdmissionFailure {
    AllocationUnavailable,
    MemoryBuild,
    CpuPreparation,
    RuntimeRegistration,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct RuntimeAdmissionRecord {
    pub id: RuntimeAdmissionId,
    pub requester: AgentId,
    pub authority: CapabilityId,
    pub target: AgentId,
    pub task: TaskId,
    pub image: AgentImageId,
    pub resource: ResourceId,
    pub status: RuntimeAdmissionStatus,
    pub failure: Option<RuntimeAdmissionFailure>,
}

impl RuntimeAdmissionRecord {
    pub(crate) const fn empty() -> Self {
        Self {
            id: RuntimeAdmissionId::new(0),
            requester: AgentId::new(0),
            authority: CapabilityId::new(0),
            target: AgentId::new(0),
            task: TaskId::new(0),
            image: AgentImageId::new(0),
            resource: ResourceId::new(0),
            status: RuntimeAdmissionStatus::Requested,
            failure: None,
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct RuntimeAdmissionPermit {
    record: RuntimeAdmissionRecord,
    generation: u64,
}

impl RuntimeAdmissionPermit {
    pub(crate) const fn new(record: RuntimeAdmissionRecord, generation: u64) -> Self {
        Self { record, generation }
    }

    pub const fn admission(self) -> RuntimeAdmissionId {
        self.record.id
    }

    pub const fn requester(self) -> AgentId {
        self.record.requester
    }

    pub const fn authority(self) -> CapabilityId {
        self.record.authority
    }

    pub const fn target(self) -> AgentId {
        self.record.target
    }

    pub const fn task(self) -> TaskId {
        self.record.task
    }

    pub const fn image(self) -> AgentImageId {
        self.record.image
    }

    pub const fn resource(self) -> ResourceId {
        self.record.resource
    }

    pub(crate) const fn record(self) -> RuntimeAdmissionRecord {
        self.record
    }

    pub(crate) const fn generation(self) -> u64 {
        self.generation
    }
}
