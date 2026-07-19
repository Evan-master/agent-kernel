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
    Released,
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
pub struct RuntimeAdmissionCompaction {
    first: RuntimeAdmissionId,
    through: RuntimeAdmissionId,
    count: usize,
}

impl RuntimeAdmissionCompaction {
    pub(crate) const fn new(
        first: RuntimeAdmissionId,
        through: RuntimeAdmissionId,
        count: usize,
    ) -> Self {
        Self {
            first,
            through,
            count,
        }
    }

    pub const fn first(self) -> RuntimeAdmissionId {
        self.first
    }

    pub const fn through(self) -> RuntimeAdmissionId {
        self.through
    }

    pub const fn count(self) -> usize {
        self.count
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

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct RuntimeAdmissionReleaseBatch<const COUNT: usize> {
    records: [RuntimeAdmissionRecord; COUNT],
    generation: u64,
}

impl<const COUNT: usize> RuntimeAdmissionReleaseBatch<COUNT> {
    pub(crate) const fn new(records: [RuntimeAdmissionRecord; COUNT], generation: u64) -> Self {
        Self {
            records,
            generation,
        }
    }

    pub const fn len(&self) -> usize {
        COUNT
    }

    pub const fn is_empty(&self) -> bool {
        COUNT == 0
    }

    pub const fn records(&self) -> &[RuntimeAdmissionRecord; COUNT] {
        &self.records
    }

    pub(crate) const fn generation(&self) -> u64 {
        self.generation
    }
}
