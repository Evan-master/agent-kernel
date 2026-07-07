//! Kernel-owned agent executable identity records.
//!
//! This core-layer module defines fixed-width image metadata. It stores
//! provenance and compatibility identity only; executable bytes, loaders, and
//! hash computation stay outside the no_std kernel core.

use crate::{AgentId, AgentImageId, ResourceId};

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct AgentImageDigest {
    pub bytes: [u8; 32],
}

impl AgentImageDigest {
    pub const fn new(bytes: [u8; 32]) -> Self {
        Self { bytes }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum AgentImageKind {
    Bootstrap,
    Supervisor,
    Worker,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum AgentImageStatus {
    Pending,
    Verified,
    Retired,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct AgentImageRecord {
    pub id: AgentImageId,
    pub owner: AgentId,
    pub resource: ResourceId,
    pub kind: AgentImageKind,
    pub digest: AgentImageDigest,
    pub abi_version: u16,
    pub entry_version: u16,
    pub status: AgentImageStatus,
}

impl AgentImageRecord {
    pub(crate) const fn empty() -> Self {
        Self {
            id: AgentImageId::new(0),
            owner: AgentId::new(0),
            resource: ResourceId::new(0),
            kind: AgentImageKind::Worker,
            digest: AgentImageDigest::new([0; 32]),
            abi_version: 0,
            entry_version: 0,
            status: AgentImageStatus::Retired,
        }
    }
}
