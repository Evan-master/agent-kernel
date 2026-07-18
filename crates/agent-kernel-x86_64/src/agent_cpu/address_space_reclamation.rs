//! Completed-CPU transfer into the native address-space reclamation pool.
//!
//! This CPU-layer child keeps trusted Agent context attached to the physical
//! owner while delegating page clearing and pool mutation to Agent memory.

use agent_kernel_x86_64::{
    address_space::AGENT_OWNED_FRAME_COUNT, address_space_reclamation::AddressSpaceReclamation,
};

use super::{CompletedAgentCpu, PreparedAgentCpu};
use crate::agent_memory::{NativeAddressSpaceFramePool, ReclaimedAgentAddressSpace};

#[derive(Copy, Clone)]
pub(crate) struct ReclaimedAgentCpuAddressSpace {
    agent: agent_kernel_core::AgentId,
    root: u64,
    frame_count: usize,
}

impl CompletedAgentCpu {
    pub(crate) fn prepare_address_space_reclamation(
        &self,
        pool: &NativeAddressSpaceFramePool,
    ) -> Option<AddressSpaceReclamation> {
        self.memory.prepare_address_space_reclamation(pool)
    }

    pub(crate) fn reclaim_address_space(
        self,
        pool: &mut NativeAddressSpaceFramePool,
        reclamation: AddressSpaceReclamation,
    ) -> Option<ReclaimedAgentCpuAddressSpace> {
        let agent = self.context().agent();
        let reclaimed: ReclaimedAgentAddressSpace =
            self.memory.reclaim_address_space(pool, reclamation)?;
        Some(ReclaimedAgentCpuAddressSpace {
            agent,
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
    pub(crate) const fn matches(self, agent: agent_kernel_core::AgentId, root: u64) -> bool {
        self.agent.raw() == agent.raw()
            && self.root == root
            && self.frame_count == AGENT_OWNED_FRAME_COUNT
    }
}
