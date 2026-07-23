//! Completed-CPU transfer into the native address-space reclamation pool.
//!
//! This CPU-layer child keeps trusted Agent context attached to the physical
//! owner while delegating page clearing and pool mutation to Agent memory.

use agent_kernel_x86_64::{
    address_space::AgentMemoryIdentity,
    address_space_reclamation::AddressSpaceReclamation,
    tlb::{TlbAddressSpace, TlbShootdownCompletion},
};

use super::{CompletedAgentCpu, PreparedAgentCpu};
use crate::agent_memory::{
    NativeAddressSpaceFramePool, QuarantinedAgentAddressSpace, ReclaimedAgentAddressSpace,
};

#[derive(Copy, Clone)]
pub(crate) struct ReclaimedAgentCpuAddressSpace {
    agent: agent_kernel_core::AgentId,
    root: u64,
    frame_count: usize,
}

pub(crate) struct QuarantinedAgentCpuAddressSpace {
    agent: agent_kernel_core::AgentId,
    address_space: QuarantinedAgentAddressSpace,
}

impl CompletedAgentCpu {
    pub(crate) fn prepare_address_space_reclamation(
        &self,
        pool: &NativeAddressSpaceFramePool,
    ) -> Option<AddressSpaceReclamation> {
        self.memory.prepare_address_space_reclamation(pool)
    }

    pub(crate) fn quarantine_address_space(
        self,
        pool: &NativeAddressSpaceFramePool,
        reclamation: AddressSpaceReclamation,
    ) -> Option<QuarantinedAgentCpuAddressSpace> {
        let agent = self.context().agent();
        let address_space = self.memory.quarantine_address_space(pool, reclamation)?;
        Some(QuarantinedAgentCpuAddressSpace {
            agent,
            address_space,
        })
    }
}

impl QuarantinedAgentCpuAddressSpace {
    pub(crate) const fn tlb_address_space(&self) -> TlbAddressSpace {
        self.address_space.address_space()
    }

    pub(crate) fn reclaim_after_shootdown(
        self,
        pool: &mut NativeAddressSpaceFramePool,
        completion: TlbShootdownCompletion,
    ) -> Option<ReclaimedAgentCpuAddressSpace> {
        let reclaimed: ReclaimedAgentAddressSpace = self
            .address_space
            .reclaim_after_shootdown(pool, completion)?;
        Some(ReclaimedAgentCpuAddressSpace {
            agent: self.agent,
            root: reclaimed.root(),
            frame_count: reclaimed.frame_count(),
        })
    }
}

impl PreparedAgentCpu {
    pub(crate) fn reclaim_unstarted_address_space(
        self,
        pool: &mut NativeAddressSpaceFramePool,
    ) -> Option<ReclaimedAgentCpuAddressSpace> {
        let agent = self.context().agent();
        let reclaimed = self.memory.cancel_address_space(pool)?;
        Some(ReclaimedAgentCpuAddressSpace {
            agent,
            root: reclaimed.root(),
            frame_count: reclaimed.frame_count(),
        })
    }
}

impl ReclaimedAgentCpuAddressSpace {
    pub(crate) const fn matches(
        self,
        agent: agent_kernel_core::AgentId,
        identity: AgentMemoryIdentity,
    ) -> bool {
        self.agent.raw() == agent.raw()
            && self.root == identity.root()
            && self.frame_count == identity.owned_frame_count()
    }
}
