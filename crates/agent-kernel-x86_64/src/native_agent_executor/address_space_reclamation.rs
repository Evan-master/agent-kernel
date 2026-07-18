//! Batch reclamation for every completed native CPU address-space owner.
//!
//! This executor child preflights all pool transitions on a copied ledger,
//! then consumes completed CPUs in the same order. Semantic transcript checks
//! run earlier while the completion report still owns every address space.

use agent_kernel_core::AgentId;
use agent_kernel_x86_64::address_space_reclamation::AddressSpaceReclamation;

use super::NativeExecutionReport;
use crate::agent_memory::{
    NativeAddressSpaceFramePool, NATIVE_ADDRESS_SPACE_CAPACITY, NATIVE_ADDRESS_SPACE_FRAME_CAPACITY,
};

impl NativeExecutionReport {
    pub(crate) fn reclaim_completed_address_spaces(
        &mut self,
        pool: &mut NativeAddressSpaceFramePool,
        agents: [AgentId; NATIVE_ADDRESS_SPACE_CAPACITY],
    ) -> Option<()> {
        if self.completed.len() != NATIVE_ADDRESS_SPACE_CAPACITY
            || !self.faulted.is_empty()
            || pool.len() != 0
        {
            return None;
        }

        let mut preview = *pool;
        let mut tokens: [Option<AddressSpaceReclamation>; NATIVE_ADDRESS_SPACE_CAPACITY] =
            [None; NATIVE_ADDRESS_SPACE_CAPACITY];
        let mut roots = [0; NATIVE_ADDRESS_SPACE_CAPACITY];
        for (index, agent) in agents.iter().copied().enumerate() {
            if agent.raw() == 0 || agents[..index].contains(&agent) {
                return None;
            }
            let cpu = self.completed.get(agent).ok()?;
            let token = cpu.prepare_address_space_reclamation(&preview)?;
            roots[index] = token.identity().root();
            if !preview.preview_commit(token) {
                return None;
            }
            tokens[index] = Some(token);
        }
        if preview.len() != NATIVE_ADDRESS_SPACE_FRAME_CAPACITY {
            return None;
        }

        for (index, agent) in agents.iter().copied().enumerate() {
            let cpu = self.completed.take(agent).ok()?;
            let reclaimed = cpu.reclaim_address_space(pool, tokens[index]?)?;
            if !reclaimed.matches(agent, roots[index]) {
                return None;
            }
        }
        (self.completed.is_empty()
            && pool.len() == NATIVE_ADDRESS_SPACE_FRAME_CAPACITY
            && pool.all_reclaimed_and_zero())
        .then_some(())
    }
}
