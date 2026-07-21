//! Read-only evidence accessors for address-space ownership transitions.
//!
//! Constructors and generation fields remain private to the parent ledger.
//! This child exposes only Agent, identity, and exact frame-count evidence.

use crate::address_space::AgentMemoryIdentity;

use super::{AddressSpaceAllocation, AddressSpaceReclamation, AllocatedAddressSpaceFrames};

impl AddressSpaceReclamation {
    pub const fn identity(self) -> AgentMemoryIdentity {
        self.identity
    }

    pub const fn frame_count(self) -> usize {
        self.identity.owned_frame_count()
    }
}

impl AddressSpaceAllocation {
    pub const fn agent(self) -> agent_kernel_core::AgentId {
        self.agent
    }

    pub const fn identity(self) -> AgentMemoryIdentity {
        self.identity
    }
}

impl AllocatedAddressSpaceFrames {
    pub const fn agent(&self) -> agent_kernel_core::AgentId {
        self.agent
    }

    pub const fn identity(&self) -> AgentMemoryIdentity {
        self.identity
    }

    pub const fn into_identity(self) -> AgentMemoryIdentity {
        self.identity
    }
}
