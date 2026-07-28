//! Architecture-neutral DMA authority records.
//!
//! This core module defines fixed-width domain, requester, attachment, and
//! mapping state. Physical addresses, PCI coordinates, and IOMMU table formats
//! remain owned by architecture crates.

use crate::{AgentId, DmaMappingId, ResourceId};

pub const DMA_PAGE_BYTES: u64 = 4096;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum DmaAccess {
    Read,
    Write,
    ReadWrite,
}

impl DmaAccess {
    pub const fn device_can_read(self) -> bool {
        matches!(self, Self::Read | Self::ReadWrite)
    }

    pub const fn device_can_write(self) -> bool {
        matches!(self, Self::Write | Self::ReadWrite)
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct DmaRequesterId(u32);

impl DmaRequesterId {
    pub const fn new(raw: u32) -> Self {
        Self(raw)
    }

    pub const fn raw(self) -> u32 {
        self.0
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct DmaDomainRecord {
    pub resource: ResourceId,
    pub iommu: ResourceId,
    pub owner: AgentId,
}

impl DmaDomainRecord {
    pub(crate) const fn empty() -> Self {
        Self {
            resource: ResourceId::new(0),
            iommu: ResourceId::new(0),
            owner: AgentId::new(0),
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum DmaAttachmentStatus {
    Attached,
    Detaching,
    Detached,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct DmaAttachmentRecord {
    pub domain: ResourceId,
    pub device: ResourceId,
    pub requester: DmaRequesterId,
    pub status: DmaAttachmentStatus,
}

impl DmaAttachmentRecord {
    pub const fn status(self) -> DmaAttachmentStatus {
        self.status
    }

    pub const fn occupies_attachment(self) -> bool {
        !matches!(self.status, DmaAttachmentStatus::Detached)
    }

    pub(crate) const fn empty() -> Self {
        Self {
            domain: ResourceId::new(0),
            device: ResourceId::new(0),
            requester: DmaRequesterId::new(0),
            status: DmaAttachmentStatus::Detached,
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum DmaMappingStatus {
    Reserved,
    Active,
    Revoking,
    Released,
    Cancelled,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct DmaMappingRecord {
    pub id: DmaMappingId,
    pub domain: ResourceId,
    pub memory: ResourceId,
    pub iova: u64,
    pub page_count: u32,
    pub access: DmaAccess,
    pub status: DmaMappingStatus,
}

impl DmaMappingRecord {
    pub const fn end_iova(self) -> Option<u64> {
        match (self.page_count as u64).checked_mul(DMA_PAGE_BYTES) {
            Some(bytes) => self.iova.checked_add(bytes),
            None => None,
        }
    }

    pub const fn occupies_iova(self) -> bool {
        !matches!(
            self.status,
            DmaMappingStatus::Released | DmaMappingStatus::Cancelled
        )
    }

    pub(crate) const fn empty() -> Self {
        Self {
            id: DmaMappingId::new(0),
            domain: ResourceId::new(0),
            memory: ResourceId::new(0),
            iova: 0,
            page_count: 0,
            access: DmaAccess::Read,
            status: DmaMappingStatus::Cancelled,
        }
    }
}
