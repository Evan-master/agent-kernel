//! Evidence accessors for native address-space service transitions.

use agent_kernel_core::AgentId;
use agent_kernel_x86_64::address_space::AgentMemoryIdentity;

use super::{
    NativeAddressSpaceAdmission, NativeAddressSpaceAdmissionFailure,
    NativeAddressSpaceAdmissionStage,
};

impl NativeAddressSpaceAdmission {
    pub(crate) const fn agent(self) -> AgentId {
        self.agent
    }

    pub(crate) const fn identity(self) -> AgentMemoryIdentity {
        self.identity
    }

    pub(crate) fn is_disjoint_from(self, other: Self) -> bool {
        self.identity.is_disjoint_from(other.identity)
    }
}

impl NativeAddressSpaceAdmissionFailure {
    pub(super) const fn allocation(agent: AgentId) -> Self {
        Self {
            stage: NativeAddressSpaceAdmissionStage::Allocation,
            agent,
            identity: None,
            rolled_back: false,
        }
    }

    pub(super) const fn rolled_back(
        stage: NativeAddressSpaceAdmissionStage,
        agent: AgentId,
        identity: AgentMemoryIdentity,
    ) -> Self {
        Self {
            stage,
            agent,
            identity: Some(identity),
            rolled_back: true,
        }
    }

    pub(crate) fn proves_rollback(
        self,
        stage: NativeAddressSpaceAdmissionStage,
        agent: AgentId,
        identity: AgentMemoryIdentity,
    ) -> bool {
        self.rolled_back
            && self.stage == stage
            && self.agent.raw() == agent.raw()
            && matches!(self.identity, Some(candidate) if candidate == identity)
    }

    pub(crate) const fn identity(self) -> Option<AgentMemoryIdentity> {
        self.identity
    }
}
